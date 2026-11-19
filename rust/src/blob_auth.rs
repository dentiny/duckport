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
    s3: Vec<Scoped<StorageConfig>>,
    huggingface: Vec<Scoped<StorageConfig>>,
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
                sources.s3.push(Scoped {
                    scope: unsafe { utils::string(secret.scope) }?,
                    value: config,
                });
            }
            "huggingface" => {
                let config = unsafe { secret.config.read() }?;
                crate::provider_auth::reject_http_headers(config.security.http()?, "Hugging Face")?;
                sources.huggingface.push(Scoped {
                    scope: unsafe { utils::string(secret.scope) }?,
                    value: config,
                });
            }
            _ => return Err("unsupported Blob source secret type".into()),
        }
    }
    Ok(sources)
}

struct Scoped<T> {
    scope: String,
    value: T,
}

// Match DuckDB's prefix-based scopes, preserving the first entry on a tie.
fn select<'a, T>(secrets: &'a [Scoped<T>], source: &str) -> Option<&'a T> {
    let mut best: Option<&Scoped<T>> = None;
    for secret in secrets {
        if source.starts_with(&secret.scope)
            && best.is_none_or(|old| secret.scope.len() > old.scope.len())
        {
            best = Some(secret);
        }
    }
    best.map(|secret| &secret.value)
}

// The transport owns only prepared headers, so redirect hops never parse raw
// credentials or depend on provider/storage configuration.
struct CredentialTransport {
    credentials: Vec<Scoped<http::HeaderMap>>,
    inner: HttpTransporter,
}
fn transport(credentials: Arc<Vec<ScopedHttpCredentials>>) -> Result<HttpTransporter> {
    let credentials = credentials
        .iter()
        .map(|secret| {
            Ok(Scoped {
                scope: secret.scope.clone(),
                value: secret.credentials.download_headers()?,
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
    Ok(BlobDownloader::with_defaults()?
        .with_io_request_size(raw_policy.io_request_size()?)
        .with_downloader(Arc::new(backend)))
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
    sources: Vec<Scoped<StorageConfig>>,
    huggingface: Vec<Scoped<StorageConfig>>,
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
        let storage = if let Some(config) = select(&self.sources, source) {
            let (operator, _) = crate::storage::operator(source, config).map_err(Error::Backend)?;
            open_arrow_sink_storage::Storage::for_url(operator, &url)?
        } else {
            open_arrow_sink_storage::Storage::from_url(&url)?.with_config(self.policy.clone())?
        };
        self.fallback
            .open(source, context.with_source_storage(storage))
            .await
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
        let policy = if let Some(config) = select(&self.huggingface, source) {
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
            .open(ByteReadRequest::new(decode(file)?).with_options(
                open_arrow_access::ByteReadOptions {
                    io_chunk_size: context.io_request_size(),
                    ..Default::default()
                },
            ))
            .await
            .map_err(Error::backend)?;
        Ok(Box::pin(
            open_arrow_access::into_stream(reader).map_err(Error::backend),
        ))
    }
}
