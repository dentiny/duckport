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

pub(crate) fn required<T>(value: Option<T>, message: &str) -> Result<T> {
    value.ok_or_else(|| message.into())
}

pub(crate) fn non_null<T>(value: *const T, message: &str) -> Result<()> {
    if value.is_null() {
        return Err(message.into());
    }
    Ok(())
}

/// # Safety
/// A non-null pointer must reference a live NUL-terminated string.
pub(crate) unsafe fn string(value: *const std::ffi::c_char) -> Result<String> {
    non_null(value, "null string argument")?;
    Ok(unsafe { std::ffi::CStr::from_ptr(value) }
        .to_str()?
        .to_owned())
}

/// # Safety
/// A non-null pointer must reference a live NUL-terminated string.
pub(crate) unsafe fn optional_string(value: *const std::ffi::c_char) -> Result<Option<String>> {
    if value.is_null() {
        Ok(None)
    } else {
        unsafe { string(value) }.map(Some)
    }
}

/// # Safety
/// For nonzero length, the pointer must reference len readable, aligned values.
pub(crate) unsafe fn ffi_slice<'a, T>(
    value: *const T,
    len: usize,
    message: &str,
) -> Result<&'a [T]> {
    if len == 0 {
        return Ok(&[]);
    }
    non_null(value, message)?;
    Ok(unsafe { std::slice::from_raw_parts(value, len) })
}

pub(crate) fn lance_mode(mode: &str) -> Result<open_arrow_sink_core::WriteMode> {
    use open_arrow_sink_core::WriteMode;
    match mode {
        "create" => Ok(WriteMode::Create),
        "append" => Ok(WriteMode::Append),
        "overwrite" => Ok(WriteMode::Overwrite),
        _ => Err("Lance mode must be create, append, or overwrite".into()),
    }
}

pub(crate) fn lance_url(path: &str) -> Result<Url> {
    let mut url = if path.contains("://") {
        Url::parse(path)?
    } else {
        Url::from_directory_path(local_path(path)?).map_err(|_| "invalid dataset path")?
    };
    validate_uri(&url)?;
    if !matches!(url.scheme(), "file" | "s3") {
        return Err("Lance destination must be a local path, file:// URI, or s3:// URI".into());
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

pub(crate) fn validate_requester_pays(
    locator: &str,
    config: &StorageConfig,
    enabled: bool,
) -> Result<()> {
    if enabled && !locator.starts_with("s3://") {
        return Err("requester_pays requires an S3 root".into());
    }
    if enabled && (config.access_key_id.is_none() || config.secret_access_key.is_none()) {
        return Err("requester_pays requires a scoped S3 secret with credentials".into());
    }
    Ok(())
}

pub(crate) fn validate_oai_selectors(
    locator: &str,
    from: Option<&str>,
    until: Option<&str>,
    set: Option<&str>,
) -> Result<()> {
    if locator != "records" && (from.is_some() || until.is_some() || set.is_some()) {
        return Err("resumption tokens cannot be combined with date/set selectors".into());
    }
    Ok(())
}

pub(crate) fn validate_hathifiles_locator(locator: &str) -> Result<()> {
    if locator != "catalog" {
        return Err("Hathifiles locator must be catalog".into());
    }
    Ok(())
}

pub(crate) fn internet_archive_credentials(config: &StorageConfig) -> Result<Option<(&str, &str)>> {
    match (
        config.access_key_id.as_deref(),
        config.secret_access_key.as_deref(),
    ) {
        (Some(key), Some(secret)) => Ok(Some((key, secret))),
        (None, None) => Ok(None),
        _ => Err("Internet Archive requires both an access key and a secret key".into()),
    }
}

pub(crate) fn courtlistener_header(
    header: &csv_async::StringRecord,
) -> Result<arrow_schema::Schema> {
    let mut names = std::collections::HashSet::new();
    let mut fields = Vec::new();
    for name in header {
        if name.is_empty() || !names.insert(name.to_owned()) {
            return Err("CourtListener CSV header names must be nonempty and unique".into());
        }
        fields.push(arrow_schema::Field::new(
            name,
            arrow_schema::DataType::Utf8,
            true,
        ));
    }
    Ok(arrow_schema::Schema::new(fields))
}

pub(crate) fn local_path(locator: &str) -> Result<std::path::PathBuf> {
    let path = if locator.starts_with("file://") {
        Url::parse(locator)?
            .to_file_path()
            .map_err(|_| "invalid local file URI")?
    } else {
        std::path::Path::new(locator).to_path_buf()
    };
    Ok(std::path::absolute(path)?)
}

// The service takes a typed request; adapt a fully qualified split URI at the SQL boundary.
pub(crate) fn huggingface_request(
    locator: &str,
) -> Result<open_arrow_access::service::huggingface::HuggingFaceDatasetRequest> {
    use open_arrow_access::service::huggingface::HuggingFaceDatasetRequest;
    let url = url::Url::parse(locator)?;
    crate::validation::validate_uri(&url)?;
    if url.scheme() != "hf" || url.host_str() != Some("datasets") || url.port().is_some() {
        return Err("expected hf://datasets/<owner>/<repo>@<revision>/<config>/<split>/".into());
    }
    let path = locator
        .strip_prefix("hf://datasets/")
        .ok_or("expected a fully qualified hf://datasets/ URL")?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let parts = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
    let [owner, repository, config, split] = parts.as_slice() else {
        return Err("Hugging Face URL must select a config/split directory".into());
    };
    let (repository, revision) = repository
        .split_once('@')
        .ok_or("Hugging Face URL must include @<revision>")?;
    let decode = |value: &str| -> Result<String> {
        Ok(percent_encoding::percent_decode_str(value)
            .decode_utf8()?
            .into_owned())
    };
    let owner = decode(owner)?;
    let repository = decode(repository)?;
    let revision = decode(revision)?;
    let config = decode(config)?;
    let split = decode(split)?;
    crate::validation::validate_huggingface_path(&owner, &repository, &revision, &config, &split)?;
    Ok(
        HuggingFaceDatasetRequest::new(format!("{owner}/{repository}"), config, split)
            .with_revision(if revision == "~parquet" {
                "refs/convert/parquet".into()
            } else {
                revision
            }),
    )
}

pub(crate) fn attachment_options(
    format: &str,
    record_path: Option<&str>,
) -> Result<open_arrow_access::AttachmentOptions> {
    use open_arrow_access::{AttachmentFormat, AttachmentOptions, OuterCompression};
    let selection = match format {
        "binary" => AttachmentFormat::Binary,
        "csv" => AttachmentFormat::Csv,
        "tsv" => AttachmentFormat::Tsv,
        "json" => AttachmentFormat::Json,
        "jsonl" => AttachmentFormat::Jsonl,
        "parquet" => AttachmentFormat::Parquet,
        "xml" => AttachmentFormat::Xml {
            record_path: record_path
                .filter(|path| !path.is_empty())
                .ok_or("record_path is required for XML attachments")?
                .to_owned(),
        },
        _ => return Err("format must be binary, csv, tsv, json, jsonl, parquet, or xml".into()),
    };
    if format != "xml" && record_path.is_some() {
        return Err("record_path is only supported for XML attachments".into());
    }
    let compression = if format == "binary" {
        OuterCompression::None
    } else {
        OuterCompression::Auto
    };
    let options = AttachmentOptions::new(selection).with_outer_compression(compression);
    options.validate()?;
    Ok(options)
}

pub(crate) fn arxiv_kind(value: &str) -> Result<open_arrow_access::service::arxiv::ArxivKind> {
    use open_arrow_access::service::arxiv::ArxivKind;
    match value {
        "source" => Ok(ArxivKind::Source),
        "pdf" => Ok(ArxivKind::Pdf),
        _ => Err("kind must be source or pdf".into()),
    }
}

pub(crate) fn biorxiv_server(value: &str) -> Result<open_arrow_access::service::biorxiv::Server> {
    use open_arrow_access::service::biorxiv::Server;
    match value {
        "biorxiv" => Ok(Server::Biorxiv),
        "medrxiv" => Ok(Server::Medrxiv),
        _ => Err("server must be biorxiv or medrxiv".into()),
    }
}

pub(crate) fn regulations_resource(
    value: &str,
) -> Result<open_arrow_access::service::regulations::RegulationsResource> {
    use open_arrow_access::service::regulations::RegulationsResource;
    match value {
        "documents" => Ok(RegulationsResource::Documents),
        "comments" => Ok(RegulationsResource::Comments),
        "dockets" => Ok(RegulationsResource::Dockets),
        _ => Err("resource must be documents, comments, or dockets".into()),
    }
}

pub(crate) fn commoncrawl_location(locator: &str) -> Result<(std::path::PathBuf, String)> {
    let path = local_path(locator)?;
    let root = path
        .ancestors()
        .find(|parent| parent.file_name().is_some_and(|name| name == "crawl-data"))
        .and_then(|directory| directory.parent())
        .ok_or("local Common Crawl path must contain crawl-data/<crawl>")?;
    let key = path
        .strip_prefix(root)?
        .iter()
        .map(|part| part.to_str().ok_or("path must be UTF-8"))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("/");
    Ok((root.to_path_buf(), key))
}

pub(crate) fn nonempty(value: Option<String>, message: &str) -> Result<String> {
    required(value.filter(|value| !value.is_empty()), message)
}
