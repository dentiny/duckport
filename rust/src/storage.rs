use crate::{error::Result, options::StorageConfig};
use open_arrow_access::storage::opendal;
use std::path::Path;
use url::Url;

pub fn operator(locator: &str, config: &StorageConfig) -> Result<(opendal::Operator, String)> {
    opendal::install_default();
    if is_local(locator) {
        return create_local_operator(locator);
    }
    create_remote_operator(locator, config)
}

fn create_remote_operator(
    locator: &str,
    config: &StorageConfig,
) -> Result<(opendal::Operator, String)> {
    remote_operator(locator, config, None, false)
}

fn remote_operator(
    locator: &str,
    config: &StorageConfig,
    root: Option<&str>,
    requester_pays: bool,
) -> Result<(opendal::Operator, String)> {
    let mut url = Url::parse(locator)?;
    crate::validation::validate_uri(&url)?;
    let scheme = url.scheme().to_owned();
    crate::validation::validate_storage_scheme(&scheme)?;
    let path = percent_encoding::percent_decode_str(url.path().trim_start_matches('/'))
        .decode_utf8()?
        .into_owned();
    url.set_path("/");
    let operator = if scheme == "http" || scheme == "https" {
        crate::validation::validate_http_config(config)?;
        let mut builder = opendal::services::Http::default()
            .endpoint(url.as_str().trim_end_matches('/'))
            .root(root.unwrap_or("/"));
        if let Some(token) = &config.token {
            builder = builder.token(token);
        }
        opendal::Operator::new(builder)?
    } else {
        let mut builder = opendal::services::S3::default()
            .root(root.unwrap_or("/"))
            .bucket(crate::validation::required(
                url.host_str(),
                "S3 URI must name a bucket",
            )?)
            .region(config.region.as_deref().unwrap_or("us-east-1"))
            .disable_config_load()
            .disable_ec2_metadata();
        if let Some(endpoint) = &config.endpoint {
            builder = builder.endpoint(endpoint);
        }
        if let Some(key) = &config.access_key_id {
            builder = builder.access_key_id(key);
        }
        if let Some(secret) = &config.secret_access_key {
            builder = builder.secret_access_key(secret);
        }
        if let Some(token) = &config.session_token {
            builder = builder.session_token(token);
        }
        if requester_pays {
            builder = builder.enable_request_payer();
        }
        if config.virtual_host_style {
            builder = builder.enable_virtual_host_style();
        }
        if config.allow_anonymous {
            builder = builder.skip_signature();
        }
        opendal::Operator::new(builder)?
    };
    Ok((operator, path))
}

pub(crate) fn is_local(locator: &str) -> bool {
    !locator.contains("://") || locator.starts_with("file://")
}

pub(crate) fn create_local_directory_operator(root: &Path) -> Result<opendal::Operator> {
    let root = crate::validation::required(root.to_str(), "path must be UTF-8")?;
    Ok(opendal::Operator::new(
        opendal::services::Fs::default().root(root),
    )?)
}

fn create_local_operator(locator: &str) -> Result<(opendal::Operator, String)> {
    let path = crate::utils::local_path(locator)?;
    let root = crate::validation::required(path.parent(), "source must name a file")?;
    let name = crate::validation::required(
        path.file_name().and_then(|s| s.to_str()),
        "source must name a UTF-8 file",
    )?
    .into();
    Ok((create_local_directory_operator(root)?, name))
}

/// Build storage rooted above a provider-owned namespace such as bulk-data/ or pdf/.
pub(crate) fn directory_operator(
    locator: &str,
    config: &StorageConfig,
) -> Result<opendal::Operator> {
    directory_operator_with_requester_pays(locator, config, false)
}

pub(crate) fn directory_operator_with_requester_pays(
    locator: &str,
    config: &StorageConfig,
    requester_pays: bool,
) -> Result<opendal::Operator> {
    crate::validation::validate_requester_pays(locator, config, requester_pays)?;
    opendal::install_default();
    if is_local(locator) {
        return create_local_directory_operator(&crate::utils::local_path(locator)?);
    }
    let url = Url::parse(locator)?;
    crate::validation::validate_uri(&url)?;
    let root = percent_encoding::percent_decode_str(url.path())
        .decode_utf8()?
        .into_owned();
    Ok(remote_operator(locator, config, Some(&root), requester_pays)?.0)
}

pub(crate) struct SinkStorage {
    pub(crate) url: Url,
    pub(crate) storage: open_arrow_sink_storage::Storage,
}

/// Match the operator's local parent or S3 bucket-root namespace for directory sinks.
pub(crate) fn sink_storage(path: &str, config: &StorageConfig) -> Result<SinkStorage> {
    let url = crate::utils::sink_url(path)?;
    let (operator, _) = operator(url.as_str(), config)?;
    let mut root = url.join("..")?;
    if url.scheme() == "s3" {
        root.set_path("/");
    }
    Ok(SinkStorage {
        url,
        storage: open_arrow_sink_storage::Storage::new(operator, root)?,
    })
}
