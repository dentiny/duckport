use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig, optional_string},
};
use open_arrow_access::service::common::Connection;
use open_arrow_access::service::regulations::{RegulationsResource, RegulationsService};
use std::ffi::c_char;

#[repr(C)]
pub struct RegulationsOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub resource: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_regulations_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &RegulationsOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let file_name = unsafe { optional_string(reader_options.file_name) }?
            .filter(|s| !s.is_empty())
            .ok_or("file_name is required")?;
        let format =
            unsafe { optional_string(reader_options.format) }?.unwrap_or_else(|| "binary".into());
        let record_path = unsafe { optional_string(reader_options.record_path) }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.regulations.gov".into());
        let resource = unsafe { optional_string(reader_options.resource) }?
            .unwrap_or_else(|| "documents".into());
        crate::service_files::validate_format(&format, record_path.as_deref())?;
        let resource = match resource.as_str() {
            "documents" => RegulationsResource::Documents,
            "comments" => RegulationsResource::Comments,
            "dockets" => RegulationsResource::Dockets,
            _ => return Err("resource must be documents, comments, or dockets".into()),
        };
        let connection = Connection::new(&endpoint)?;
        let token = storage_options
            .token
            .filter(|s| !s.is_empty())
            .ok_or("a scoped HTTP bearer_token secret is required for this service")?;
        let connection = connection.with_header("X-Api-Key", &token)?;
        let files = RegulationsService::with_connection(connection)
            .record(resource, &locator)
            .await?;
        crate::service_files::open(files, file_name, &format, record_path, scan_options).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
