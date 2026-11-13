use crate::error::Result;
use std::collections::HashMap;
use url::Url;

pub(crate) fn validate_uri(url: &Url) -> Result<()> {
    if url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("URI queries, fragments, and embedded credentials are unsupported".into());
    }
    Ok(())
}

pub(crate) fn validate_http_config(config: &HashMap<String, String>) -> Result<()> {
    if config.contains_key("endpoint") {
        return Err("HTTP URI already determines endpoint".into());
    }
    Ok(())
}
