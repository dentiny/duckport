use crate::{error::Result, options::StorageConfig};
use open_arrow_access::storage::opendal;
use std::path::Path;
use url::Url;

pub fn operator(locator: &str, config: &StorageConfig) -> Result<(opendal::Operator, String)> {
    opendal::install_default();
    if !locator.contains("://") || locator.starts_with("file://") {
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
            .bucket(url.host_str().ok_or("S3 URI must name a bucket")?);
        if let Some(endpoint) = &config.endpoint {
            builder = builder.endpoint(endpoint);
        }
        if let Some(region) = &config.region {
            builder = builder.region(region);
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

fn create_local_operator(locator: &str) -> Result<(opendal::Operator, String)> {
    let path = if locator.starts_with("file://") {
        Url::parse(locator)?
            .to_file_path()
            .map_err(|_| "invalid local file URI")?
    } else {
        Path::new(locator).to_path_buf()
    };
    let path = std::path::absolute(path)?;
    let root = path
        .parent()
        .ok_or("source must name a file")?
        .to_str()
        .ok_or("path must be UTF-8")?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("source must name a UTF-8 file")?
        .into();
    Ok((
        opendal::Operator::new(opendal::services::Fs::default().root(root))?,
        name,
    ))
}
