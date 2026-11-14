use crate::{error::Result, options::StorageConfig};
use open_arrow_access::storage::opendal;
use std::path::{Path, PathBuf};
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
        let mut builder =
            opendal::services::Http::default().endpoint(url.as_str().trim_end_matches('/'));
        if let Some(token) = &config.token {
            builder = builder.token(token);
        }
        opendal::Operator::new(builder)?
    } else {
        let mut builder = opendal::services::S3::default()
            .bucket(url.host_str().ok_or("S3 URI must name a bucket")?)
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

pub(crate) fn local_path(locator: &str) -> Result<PathBuf> {
    let path = if locator.starts_with("file://") {
        Url::parse(locator)?
            .to_file_path()
            .map_err(|_| "invalid local file URI")?
    } else {
        Path::new(locator).to_path_buf()
    };
    Ok(std::path::absolute(path)?)
}

pub(crate) fn create_local_directory_operator(root: &Path) -> Result<opendal::Operator> {
    let root = root.to_str().ok_or("path must be UTF-8")?;
    Ok(opendal::Operator::new(
        opendal::services::Fs::default().root(root),
    )?)
}

fn create_local_operator(locator: &str) -> Result<(opendal::Operator, String)> {
    let path = local_path(locator)?;
    let root = path.parent().ok_or("source must name a file")?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("source must name a UTF-8 file")?
        .into();
    Ok((create_local_directory_operator(root)?, name))
}
