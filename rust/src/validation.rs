use crate::error::Result;
use crate::options::StorageConfig;
use std::num::NonZeroUsize;
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

pub(crate) fn validate_storage_scheme(scheme: &str) -> Result<()> {
    if !matches!(scheme, "http" | "https" | "s3") {
        return Err(format!("unsupported storage scheme: {scheme}").into());
    }
    Ok(())
}

pub(crate) fn validate_http_config(config: &StorageConfig) -> Result<()> {
    if config.endpoint.is_some() {
        return Err("HTTP URI already determines endpoint".into());
    }
    Ok(())
}

pub(crate) fn batch_size(value: usize) -> Result<NonZeroUsize> {
    NonZeroUsize::new(value)
        .filter(|size| size.get() <= 1048576)
        .ok_or_else(|| "batch_size must be between 1 and 1048576".into())
}

pub(crate) fn boolean(name: &str, value: i32) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(format!("{name} must be 0 or 1").into()),
    }
}

pub(crate) fn validate_huggingface_path(
    owner: &str,
    repository: &str,
    revision: &str,
    config: &str,
    split: &str,
) -> Result<()> {
    if [owner, repository, config, split].iter().any(|value| {
        value.is_empty() || matches!(*value, "." | "..") || value.contains(['/', '\\', '\0'])
    }) || revision.is_empty()
    {
        return Err("Hugging Face URL contains an invalid path component".into());
    }
    Ok(())
}

pub(crate) fn validate_huggingface_url_options(
    revision: Option<&str>,
    config: Option<&str>,
    split: Option<&str>,
) -> Result<()> {
    if revision.is_some() || config.is_some() || split.is_some() {
        return Err("revision, config, and split cannot be combined with a fully qualified Hugging Face URL".into());
    }
    Ok(())
}

pub(crate) fn validate_format(format: &str, record_path: Option<&str>) -> Result<()> {
    if !matches!(
        format,
        "binary" | "csv" | "tsv" | "json" | "jsonl" | "parquet" | "xml"
    ) {
        return Err("format must be binary, csv, tsv, json, jsonl, parquet, or xml".into());
    }
    if format == "xml" && record_path.is_none_or(str::is_empty) {
        return Err("record_path is required for XML attachments".into());
    }
    if format != "xml" && record_path.is_some() {
        return Err("record_path is only supported for XML attachments".into());
    }
    Ok(())
}
