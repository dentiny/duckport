use crate::{
    error::Result,
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig, optional_string},
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::service::huggingface::*;
use std::ffi::c_char;

// The service takes a typed request; adapt a fully qualified split URI at the SQL boundary.
fn dataset_request(locator: &str) -> Result<HuggingFaceDatasetRequest> {
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

#[repr(C)]
pub struct HuggingFaceDatasetOptions {
    pub revision: *const c_char,
    pub config: *const c_char,
    pub split: *const c_char,
    pub allow_partial: i32,
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_huggingface_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &HuggingFaceDatasetOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let revision = unsafe { optional_string(reader_options.revision) }?;
        let config = unsafe { optional_string(reader_options.config) }?;
        let split = unsafe { optional_string(reader_options.split) }?;
        let allow_partial =
            crate::validation::boolean("allow_partial", reader_options.allow_partial)?;

        let mut request = if locator.contains("://") {
            crate::validation::validate_huggingface_url_options(
                revision.as_deref(),
                config.as_deref(),
                split.as_deref(),
            )?;
            dataset_request(&locator)?
        } else {
            let mut request = HuggingFaceDatasetRequest::new(
                &locator,
                config.unwrap_or_else(|| "default".into()),
                split.unwrap_or_else(|| "train".into()),
            );
            if let Some(revision) = revision {
                request = request.with_revision(revision);
            }
            request
        };
        if allow_partial {
            request = request.with_partial_exports();
        }

        let mut settings = HuggingFaceOptions::default();
        if let Some(endpoint) = storage_options.endpoint {
            settings = settings.with_endpoint(endpoint);
        }
        if let Some(token) = storage_options.token {
            settings = settings.with_token(token);
        }
        let service = HuggingFaceService::new(settings).await?;
        AsyncSource::new(service, request.with_options(scan_options)).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
