//! Owned credential snapshots for asynchronous source downloads. No worker calls
//! DuckDB, and no credential or URL is included in transport error messages.
use crate::{
    error::{Result, http_source_error},
    options::DuckportStorageOptions,
    security::ScopedHttpCredentials,
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
pub(crate) unsafe fn read_secrets(
    raw: *const DuckportBlobSecret,
    count: usize,
) -> Result<Vec<ScopedHttpCredentials>> {
    unsafe { utils::ffi_slice(raw, count, "null Blob secrets") }?
        .iter()
        .map(|s| unsafe {
            if utils::string(s.r#type)? != "http" {
                return Err("unsupported Blob source secret type".into());
            }
            crate::security::read_http_secret(s.scope, &s.config.security)
        })
        .collect()
}
// The transport owns only prepared headers, so redirect hops never parse raw
// credentials or depend on provider/storage configuration.
struct HttpSourceCredentials {
    scope: String,
    headers: http::HeaderMap,
}
// Match DuckDB's prefix-based scopes, preserving the first entry on a tie.
fn select<'a>(secrets: &'a [HttpSourceCredentials], url: &str) -> Option<&'a http::HeaderMap> {
    let mut best: Option<&HttpSourceCredentials> = None;
    for secret in secrets {
        if url.starts_with(&secret.scope)
            && best.is_none_or(|old| secret.scope.len() > old.scope.len())
        {
            best = Some(secret);
        }
    }
    best.map(|s| &s.headers)
}
struct CredentialTransport {
    credentials: Vec<HttpSourceCredentials>,
    inner: HttpTransporter,
}
fn transport(credentials: Arc<Vec<ScopedHttpCredentials>>) -> Result<HttpTransporter> {
    let credentials = credentials
        .iter()
        .map(|secret| {
            Ok(HttpSourceCredentials {
                scope: secret.scope.clone(),
                headers: secret.credentials.download_headers()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
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
pub(crate) fn scoped_http_transport(
    secrets: Arc<Vec<ScopedHttpCredentials>>,
) -> Result<HttpTransporter> {
    transport(secrets)
}
pub(crate) fn blob_downloader(
    secrets: Vec<ScopedHttpCredentials>,
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
    let backend =
        OpenDalDownloader::new(policy)?.with_http_transport(transport(Arc::new(secrets))?);
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
                if let Some(headers) = select(&self.credentials, url.as_str()) {
                    outgoing.extend(headers.clone());
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
