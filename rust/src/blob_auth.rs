//! Owned credential snapshots for asynchronous source downloads. No worker calls
//! DuckDB, and no credential or URL is included in transport error messages.
use crate::{
    error::{Result, http_source_error},
    options::{DuckportStorageOptions, StorageConfig},
    security::{ScopedHttpSecret, SecurityConfig},
    utils, validation,
};
use http::{Request, Response};
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
pub(crate) fn select<'a>(secrets: &'a [ScopedHttpSecret], url: &str) -> Option<&'a SecurityConfig> {
    let mut best: Option<&ScopedHttpSecret> = None;
    for secret in secrets {
        if url.starts_with(&secret.scope)
            && best.is_none_or(|old| secret.scope.len() > old.scope.len())
        {
            best = Some(secret);
        }
    }
    best.map(|s| &s.security)
}
struct CredentialTransport {
    credentials: Arc<Vec<ScopedHttpSecret>>,
    inner: HttpTransporter,
}
fn transport(credentials: Arc<Vec<ScopedHttpSecret>>) -> Result<HttpTransporter> {
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
pub(crate) fn scoped_http_transport(config: &StorageConfig) -> Result<HttpTransporter> {
    transport(Arc::clone(&config.http_secrets))
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
    let backend = OpenDalDownloader::new(policy)?.with_http_transport(transport(Arc::new(
        secrets
            .into_iter()
            .filter(|s| s.kind == "http")
            .map(|s| ScopedHttpSecret {
                scope: s.scope,
                security: s.config.security,
            })
            .collect(),
    ))?);
    Ok(BlobDownloader::with_defaults()?.with_downloader(Arc::new(backend)))
}
impl HttpTransport for CredentialTransport {
    async fn fetch(&self, request: Request<Buffer>) -> opendal::Result<Response<HttpBody>> {
        let (parts, _) = request.into_parts();
        let mut url = Url::parse(&parts.uri.to_string())
            .map_err(|_| http_source_error("invalid HTTP source URL"))?;
        let original_origin = url.origin();
        // These downloads only issue GET/HEAD requests; there is no upload body
        // to replay while following a redirect.
        validation::validate_http_source_method(&parts.method)?;
        let mut crossed_origin = false;
        for hop in 0..=10 {
            validation::validate_http_source_url(&url)?;
            let mut builder = Request::builder()
                .method(parts.method.clone())
                .uri(url.as_str());
            let outgoing = builder
                .headers_mut()
                .ok_or_else(|| http_source_error("invalid HTTP request"))?;
            *outgoing = parts.headers.clone();
            outgoing.remove(http::header::AUTHORIZATION);
            if !crossed_origin {
                let auth = select(&self.credentials, url.as_str())
                    .map(validation::http_credential_headers)
                    .transpose()
                    .map_err(|_| http_source_error("invalid scoped HTTP credentials"))?;
                if let Some(auth) = auth {
                    outgoing.extend(auth);
                }
            }
            let request = builder
                .body(Buffer::new())
                .map_err(|_| http_source_error("invalid HTTP request"))?;
            let response = self
                .inner
                .fetch(request)
                .await
                .map_err(|_| http_source_error("HTTP source request failed").set_temporary())?;
            if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                return Ok(response);
            }
            if hop == 10 {
                return Err(http_source_error("too many HTTP source redirects"));
            }
            let location = response
                .headers()
                .get(http::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| http_source_error("HTTP source redirect has no valid Location"))?;
            let next = url
                .join(location)
                .map_err(|_| http_source_error("invalid HTTP redirect target"))?;
            validation::validate_http_source_redirect(&url, &next)?;
            crossed_origin |= next.origin() != original_origin;
            url = next;
            drop(response);
        }
        Err(http_source_error("too many HTTP source redirects"))
    }
}
