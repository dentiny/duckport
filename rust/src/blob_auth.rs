//! Owned credential snapshots for asynchronous source downloads. No worker calls
//! DuckDB, and no credential or URL is included in transport error messages.
use crate::{
    error::Result,
    options::{DuckportStorageOptions, StorageConfig},
    utils,
};
use http::{HeaderMap, HeaderName, HeaderValue, Request, Response};
use open_arrow_sink_storage::{BlobDownloader, OpenDalDownloader};
use opendal::{Buffer, HttpBody, HttpTransport, HttpTransporter};
use std::{ffi::c_char, sync::Arc};
use url::Url;


#[repr(C)]
pub struct DuckportBlobSecret {
    pub scope: *const c_char,
    pub r#type: *const c_char,
    pub config: DuckportStorageOptions,
}
pub(crate) struct SourceSecret {
    pub scope: String,
    pub kind: String,
    pub config: StorageConfig,
}
pub(crate) unsafe fn read_secrets(
    raw: *const DuckportBlobSecret,
    count: usize,
) -> Result<Vec<SourceSecret>> {
    unsafe { utils::ffi_slice(raw, count, "null Blob secrets") }?
        .iter()
        .map(|s| unsafe {
            Ok(SourceSecret {
                scope: utils::string(s.scope)?,
                kind: utils::string(s.r#type)?,
                config: s.config.read()?,
            })
        })
        .collect()
}
// Match DuckDB's prefix-based scopes, preserving the first entry on a tie.
pub(crate) fn select<'a>(
    secrets: &'a [SourceSecret],
    url: &str,
    kind: &str,
) -> Option<&'a StorageConfig> {
    let mut best: Option<&SourceSecret> = None;
    for secret in secrets {
        if secret.kind == kind
            && url.starts_with(&secret.scope)
            && best.is_none_or(|old| secret.scope.len() > old.scope.len())
        {
            best = Some(secret);
        }
    }
    best.map(|s| &s.config)
}
fn headers(config: &StorageConfig) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for (name, value) in &config.security.http_headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| "invalid HTTP credential header name")?;
        if matches!(
            name.as_str(),
            "host" | "content-length" | "range" | "connection" | "transfer-encoding"
        ) {
            return Err("HTTP credential headers must not override transport headers".into());
        }
        if headers.contains_key(&name) {
            return Err("duplicate HTTP credential header".into());
        }
        let mut value =
            HeaderValue::from_str(value).map_err(|_| "invalid HTTP credential header value")?;
        value.set_sensitive(true);
        headers.insert(name, value);
    }
    if let Some(token) = &config.security.token {
        if headers.contains_key(http::header::AUTHORIZATION) {
            return Err("choose BEARER_TOKEN or an Authorization header".into());
        }
        let mut value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| "invalid HTTP bearer token")?;
        value.set_sensitive(true);
        headers.insert(http::header::AUTHORIZATION, value);
    }
    Ok(headers)
}
enum Credentials {
    Sources(Arc<Vec<SourceSecret>>),
    Fixed {
        origin: url::Origin,
        headers: HeaderMap,
    },
}
struct CredentialTransport {
    credentials: Credentials,
    inner: HttpTransporter,
}
fn transport(credentials: Credentials) -> Result<HttpTransporter> {
    // Disable automatic redirects so every hop is checked before any credential
    // can be sent. OpenDAL still owns retries, timeouts and the body stream.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    Ok(HttpTransporter::new(CredentialTransport {
        credentials,
        inner: HttpTransporter::new(opendal_http_transport_reqwest::ReqwestTransport::new(
            client,
        )),
    }))
}
pub(crate) fn fixed_http_transport(url: &str, config: &StorageConfig) -> Result<HttpTransporter> {
    transport(Credentials::Fixed {
        origin: Url::parse(url)?.origin(),
        headers: headers(config)?,
    })
}
pub(crate) fn blob_downloader(
    secrets: Vec<SourceSecret>,
    policy: crate::storage_policy::DuckportStoragePolicy,
) -> Result<BlobDownloader> {
    let source = policy.config()?;
    let policy = open_arrow_sink_storage::StorageConfig {
        retry: source.retry.map(|r| open_arrow_sink_storage::RetryConfig {
            max_retries: r.max_retries,
            min_delay: r.min_delay,
            max_delay: r.max_delay,
            factor: r.factor,
            jitter: r.jitter,
        }),
        timeout: source
            .timeout
            .map(|t| open_arrow_sink_storage::TimeoutConfig {
                operation_timeout: t.operation_timeout,
                io_timeout: t.io_timeout,
            }),
    };
    let backend = OpenDalDownloader::new(policy)?
        .with_http_transport(transport(Credentials::Sources(Arc::new(secrets)))?);
    Ok(BlobDownloader::with_defaults()?.with_downloader(Arc::new(backend)))
}
fn failure(message: &'static str) -> opendal::Error {
    opendal::Error::new(opendal::ErrorKind::Unexpected, message)
}
impl HttpTransport for CredentialTransport {
    async fn fetch(&self, request: Request<Buffer>) -> opendal::Result<Response<HttpBody>> {
        let (parts, _) = request.into_parts();
        let mut url =
            Url::parse(&parts.uri.to_string()).map_err(|_| failure("invalid HTTP source URL"))?;
        let original_origin = url.origin();
        // These downloads only issue GET/HEAD requests; there is no upload body
        // to replay while following a redirect.
        if parts.method != http::Method::GET && parts.method != http::Method::HEAD {
            return Err(failure("unsupported download method"));
        }
        let mut crossed_origin = false;
        for hop in 0..=10 {
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err(failure("unsupported HTTP redirect target"));
            }
            let mut builder = Request::builder()
                .method(parts.method.clone())
                .uri(url.as_str());
            let outgoing = builder
                .headers_mut()
                .ok_or_else(|| failure("invalid HTTP request"))?;
            *outgoing = parts.headers.clone();
            outgoing.remove(http::header::AUTHORIZATION);
            if !crossed_origin {
                let auth = match &self.credentials {
                    Credentials::Sources(secrets) => select(secrets, url.as_str(), "http")
                        .map(headers)
                        .transpose()
                        .map_err(|_| failure("invalid scoped HTTP credentials"))?,
                    Credentials::Fixed { origin, headers } if *origin == url.origin() => {
                        Some(headers.clone())
                    }
                    Credentials::Fixed { .. } => None,
                };
                if let Some(auth) = auth {
                    outgoing.extend(auth);
                }
            }
            let request = builder
                .body(Buffer::new())
                .map_err(|_| failure("invalid HTTP request"))?;
            let response = self
                .inner
                .fetch(request)
                .await
                .map_err(|_| failure("HTTP source request failed").set_temporary())?;
            if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                return Ok(response);
            }
            if hop == 10 {
                return Err(failure("too many HTTP source redirects"));
            }
            let location = response
                .headers()
                .get(http::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| failure("HTTP source redirect has no valid Location"))?;
            let next = url
                .join(location)
                .map_err(|_| failure("invalid HTTP redirect target"))?;
            if url.scheme() == "https" && next.scheme() != "https" {
                return Err(failure("HTTPS source redirect downgrade rejected"));
            }
            crossed_origin |= next.origin() != original_origin;
            url = next;
            drop(response);
        }
        Err(failure("too many HTTP source redirects"))
    }
}
