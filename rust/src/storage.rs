use crate::error::Result;
use open_arrow_access::storage::opendal;
use std::{collections::HashMap, path::Path};
use url::Url;

pub(super) fn operator(
    locator: &str,
    mut config: HashMap<String, String>,
) -> Result<(opendal::Operator, String)> {
    opendal::install_default();
    if !locator.contains("://") || locator.starts_with("file://") {
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
        return Ok((
            opendal::Operator::via_iter("fs", config)?,
            path.file_name()
                .and_then(|s| s.to_str())
                .ok_or("source must name a UTF-8 file")?
                .into(),
        ));
    }
    let mut url = Url::parse(locator)?;
    if url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("URI queries, fragments, and embedded credentials are unsupported".into());
    }
    let scheme = url.scheme().to_owned();
    if !matches!(scheme.as_str(), "http" | "https" | "s3") {
        return Err(format!("unsupported storage scheme: {scheme}").into());
    }
    let path = percent_encoding::percent_decode_str(url.path().trim_start_matches('/'))
        .decode_utf8()?
        .into_owned();
    if config.contains_key("root") {
        return Err("URI already determines storage root".into());
    }
    url.set_path("/");
    let operator = if scheme == "http" || scheme == "https" {
        if config.contains_key("endpoint") {
            return Err("HTTP URI already determines endpoint".into());
        }
        config.insert("endpoint".into(), url.as_str().trim_end_matches('/').into());
        opendal::Operator::via_iter("http", config)?
    } else {
        opendal::Operator::from_uri(opendal::OperatorUri::new(url.as_str(), config)?)?
    };
    Ok((operator, path))
}
