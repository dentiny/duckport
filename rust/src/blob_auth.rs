//! Owned credential snapshots for asynchronous source downloads. No worker calls
//! DuckDB, and no credential or URL is included in transport error messages.
use crate::{
    error::{Result, http_source_error},
    options::{DuckportStorageOptions, StorageConfig},
    security::ScopedHttpCredentials,
    utils, validation,
};
use futures::TryStreamExt;
use http::{Request, Response};
use open_arrow_sink_core::{Error, async_trait};
use open_arrow_sink_storage::{
    BlobDownloader, DownloadContext, DownloadStream, Downloader, OpenDalDownloader,
};
use opendal::{Buffer, HttpBody, HttpTransport, HttpTransporter};
use std::{ffi::c_char, sync::Arc};
use url::Url;

#[repr(C)]
pub struct DuckportBlobSecret {
    pub scope: *const c_char,
    pub r#type: *const c_char,
    pub config: DuckportStorageOptions,
}
pub(crate) struct SourceSecrets {
    http: Vec<ScopedHttpCredentials>,
    s3: Vec<(String, StorageConfig)>,
    huggingface: Vec<(String, StorageConfig)>,
}

pub(crate) unsafe fn read_secrets(
    raw: *const DuckportBlobSecret,
    count: usize,
) -> Result<SourceSecrets> {
    let mut sources = SourceSecrets {
        http: Vec::new(),
        s3: Vec::new(),
        huggingface: Vec::new(),
    };
    for secret in unsafe { utils::ffi_slice(raw, count, "null Blob secrets") }? {
        match unsafe { utils::string(secret.r#type) }?.as_str() {
            "http" => sources.http.push(unsafe {
                crate::security::read_http_secret(secret.scope, &secret.config.security)
            }?),
            "s3" => {
                let config = unsafe { secret.config.read() }?;
                config.security.s3()?;
                sources
                    .s3
                    .push((unsafe { utils::string(secret.scope) }?, config));
            }
            "huggingface" => {
                let config = unsafe { secret.config.read() }?;
                crate::provider_auth::reject_http_headers(config.security.http()?, "Hugging Face")?;
                sources
                    .huggingface
                    .push((unsafe { utils::string(secret.scope) }?, config));
            }
            _ => return Err("unsupported Blob source secret type".into()),
        }
    }
    Ok(sources)
}

// Match prefix scopes, preserving the first secret on a tie.
fn select_source<'a>(
    sources: &'a [(String, StorageConfig)],
    source: &str,
) -> Option<&'a StorageConfig> {
    let mut best = None;
    let mut length = 0;
    for (scope, config) in sources {
        if source.starts_with(scope) && (best.is_none() || scope.len() > length) {
            best = Some(config);
            length = scope.len();
        }
    }
    best
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
    secrets: SourceSecrets,
    policy: crate::storage_policy::DuckportStoragePolicy,
) -> Result<BlobDownloader> {
    let raw_policy = policy;
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
    let fallback = OpenDalDownloader::new(policy.clone())?
        .with_http_transport(transport(Arc::new(secrets.http))?);
    let backend = SourceDownloader {
        fallback,
        sources: secrets.s3,
        huggingface: secrets.huggingface,
        policy,
        raw_policy,
    };
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

struct SourceDownloader {
    fallback: OpenDalDownloader,
    sources: Vec<(String, StorageConfig)>,
    huggingface: Vec<(String, StorageConfig)>,
    policy: open_arrow_sink_storage::StorageConfig,
    raw_policy: crate::storage_policy::DuckportStoragePolicy,
}
#[async_trait]
impl Downloader for SourceDownloader {
    async fn open(
        &self,
        source: &str,
        context: DownloadContext,
    ) -> open_arrow_sink_core::Result<DownloadStream> {
        let url = Url::parse(source).map_err(Error::backend)?;
        if url.scheme() == "hf" {
            return self.open_huggingface(source, &url, context).await;
        }
        if url.scheme() != "s3" {
            return self.fallback.open(source, context).await;
        }
        crate::validation::validate_uri(&url).map_err(Error::Backend)?;
        // Explicit source secrets always win, even when the source shares a
        // namespace with the destination. Never consult context.primary here.
        let (operator, path) = if let Some(config) = select_source(&self.sources, source) {
            crate::storage::operator(source, config).map_err(Error::Backend)?
        } else {
            // Preserve independent native/environment source resolution, without
            // accidentally reusing the destination's scoped operator.
            let storage = open_arrow_sink_storage::Storage::from_url(&url)?
                .with_config(self.policy.clone())?;
            (storage.operator(), storage.path(&url)?.as_str().to_owned())
        };
        stream_operator(operator, path, context).await
    }
}
impl SourceDownloader {
    async fn open_huggingface(
        &self,
        source: &str,
        url: &Url,
        context: DownloadContext,
    ) -> open_arrow_sink_core::Result<DownloadStream> {
        use open_arrow_access::{
            Accessor, ByteReadRequest,
            service::huggingface::{HuggingFaceOptions, HuggingFaceRepository, HuggingFaceService},
        };
        crate::validation::validate_uri(url).map_err(Error::Backend)?;
        if url.host_str() != Some("datasets") {
            return Err(Error::InvalidInput(
                "Hugging Face Blob sources require hf://datasets/<owner>/<repo>/<file>".into(),
            ));
        }
        let mut path = url.path().trim_start_matches('/').splitn(3, '/');
        let owner = path
            .next()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::InvalidInput("missing Hugging Face owner".into()))?;
        let repo = path
            .next()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::InvalidInput("missing Hugging Face repository".into()))?;
        let file = path
            .next()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::InvalidInput("missing Hugging Face Blob file".into()))?;
        let (repo, revision) = repo.split_once('@').unwrap_or((repo, "main"));
        let decode = |value: &str| {
            percent_encoding::percent_decode_str(value)
                .decode_utf8()
                .map(|v| v.into_owned())
                .map_err(Error::backend)
        };
        let mut options = HuggingFaceOptions::default();
        let policy = if let Some(config) = select_source(&self.huggingface, source) {
            let token = config
                .security
                .http()
                .map_err(Error::Backend)?
                .and_then(|credentials| credentials.token())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    Error::InvalidConfig("Hugging Face secret TOKEN must not be empty".into())
                })?;
            options = options.with_token(token.to_owned());
            config.policy
        } else {
            self.raw_policy
        };
        let service = HuggingFaceService::new(options)
            .await
            .map_err(Error::backend)?
            .with_http_config(policy.config().map_err(Error::Backend)?)
            .map_err(Error::backend)?;
        let repository =
            HuggingFaceRepository::new(format!("{}/{}", decode(owner)?, decode(repo)?))
                .with_revision(decode(revision)?);
        let files = service
            .repository(repository)
            .await
            .map_err(Error::backend)?
            .with_chunk_size(
                std::num::NonZeroUsize::new(context.chunk_bytes())
                    .ok_or_else(|| Error::InvalidConfig("zero Blob chunk size".into()))?,
            );
        let reader = files
            .open(ByteReadRequest::new(decode(file)?))
            .await
            .map_err(Error::backend)?;
        Ok(Box::pin(
            open_arrow_access::into_stream(reader).map_err(Error::backend),
        ))
    }
}

async fn stream_operator(
    operator: opendal::Operator,
    path: String,
    context: DownloadContext,
) -> open_arrow_sink_core::Result<DownloadStream> {
    let transport = context
        .http_requests()
        .wrap(operator.base_context().http_transport().clone());
    let operator_context = operator.base_context().with_http_transport(transport);
    let operator = operator.with_context(operator_context);
    let reader = operator
        .reader(&path)
        .await
        .map_err(Error::backend)?
        .into_stream(..)
        .await
        .map_err(Error::backend)?;
    let chunk_bytes = context.chunk_bytes();
    let chunks = reader
        .map_err(Error::backend)
        .map_ok(move |buffer| {
            futures::stream::try_unfold(buffer, move |mut buffer| async move {
                if buffer.is_empty() {
                    return Ok::<_, Error>(None);
                }
                let size = chunk_bytes.min(buffer.len());
                let chunk = open_arrow_access::Bytes::from(buffer.split_to(size).to_vec());
                Ok(Some((chunk, buffer)))
            })
        })
        .try_flatten();
    Ok(Box::pin(chunks))
}
