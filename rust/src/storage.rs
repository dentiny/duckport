use crate::error::Result;
use open_arrow_access::storage::opendal;
use std::{collections::HashMap, path::Path};
use url::Url;

pub fn operator(
    locator: &str,
    config: HashMap<String, String>,
) -> Result<(opendal::Operator, String)> {
    opendal::install_default();
    if !locator.contains("://") || locator.starts_with("file://") {
        return local_operator(locator, config);
    }
    create_remote_operator(locator, config)
}

fn create_remote_operator(
    locator: &str,
    mut config: HashMap<String, String>,
) -> Result<(opendal::Operator, String)> {
    let mut url = Url::parse(locator)?;
    crate::validation::validate_uri(&url)?;
    let scheme = url.scheme().to_owned();
    crate::validation::validate_storage_scheme(&scheme)?;
    let path = percent_encoding::percent_decode_str(url.path().trim_start_matches('/'))
        .decode_utf8()?
        .into_owned();
    crate::validation::validate_uri_config(&config)?;
    url.set_path("/");
    let operator = if scheme == "http" || scheme == "https" {
        crate::validation::validate_http_config(&config)?;
        config.insert("endpoint".into(), url.as_str().trim_end_matches('/').into());
        opendal::Operator::via_iter("http", config)?
    } else {
        opendal::Operator::from_uri(opendal::OperatorUri::new(url.as_str(), config)?)?
    };
    Ok((operator, path))
}

fn local_operator(
    locator: &str,
    mut config: HashMap<String, String>,
) -> Result<(opendal::Operator, String)> {
    if config.contains_key("root") {
        return Err("file path already determines storage root".into());
    }
    let path = if locator.starts_with("file://") {
        Url::parse(locator)?
            .to_file_path()
            .map_err(|_| "invalid local file URI")?
    } else {
        Path::new(locator).to_path_buf()
    };
    let path = std::path::absolute(path)?;
    config.insert(
        "root".into(),
        path.parent()
            .ok_or("source must name a file")?
            .to_str()
            .ok_or("path must be UTF-8")?
            .into(),
    );
    Ok((
        opendal::Operator::via_iter("fs", config)?,
        path.file_name()
            .and_then(|s| s.to_str())
            .ok_or("source must name a UTF-8 file")?
            .into(),
    ))
}
