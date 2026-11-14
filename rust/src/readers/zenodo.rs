use crate::{
    ffi::{DuckportSource, open_reader},
    options::{
        AttachmentFormat, DuckportScanOptions, DuckportStorageOptions, StorageConfig,
        optional_string,
    },
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::{
    BinaryDecoder, DocumentAccessor, ReadRequest, service::zenodo::ZenodoService,
};
use std::ffi::c_char;

#[repr(C)]
pub struct ZenodoOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_zenodo_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &ZenodoOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let file_name = unsafe { optional_string(reader_options.file_name) }?
            .filter(|s| !s.is_empty())
            .ok_or("file_name is required")?;
        let format = unsafe { optional_string(reader_options.format) }?;
        let format = match format.as_deref().unwrap_or("binary") {
            "binary" => None,
            "csv" => Some(AttachmentFormat::Csv),
            "tsv" => Some(AttachmentFormat::Tsv),
            "json" => Some(AttachmentFormat::Json),
            "jsonl" => Some(AttachmentFormat::Jsonl),
            "parquet" => Some(AttachmentFormat::Parquet),
            "xml" => Some(AttachmentFormat::Xml),
            _ => {
                return Err("format must be binary, csv, tsv, json, jsonl, parquet, or xml".into());
            }
        };
        let record_path = unsafe { optional_string(reader_options.record_path) }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://zenodo.org".into());
        if let Some(format) = format {
            crate::validation::validate_format(format, record_path.as_deref())?;
        } else if record_path.is_some() {
            return Err("record_path is only supported for XML attachments".into());
        }
        let _ = storage_options; // This upstream service currently exposes public records only.
        let files = ZenodoService::with_endpoint(&endpoint)?
            .record(locator.parse()?)
            .await?;
        match format {
            Some(format) => {
                crate::service_files::open(files, file_name, format, record_path, scan_options)
                    .await
            }
            None => {
                AsyncSource::new(
                    DocumentAccessor::new(files, BinaryDecoder),
                    ReadRequest::new(file_name).with_options(scan_options),
                )
                .await
            }
        }
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
