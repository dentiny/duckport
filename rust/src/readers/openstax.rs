use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig, optional_string},
};
use open_arrow_access::service::common::Connection;
use open_arrow_access::service::openstax::OpenStaxService;
use std::ffi::c_char;

#[repr(C)]
pub struct OpenstaxOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub revision: *const c_char,
    pub archive_endpoint: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_openstax_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &OpenstaxOptions,
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
            .unwrap_or_else(|| "https://api.github.com".into());
        let revision = unsafe { optional_string(reader_options.revision) }?
            .filter(|s| !s.is_empty())
            .ok_or("revision is required")?;
        let attachment_options =
            crate::service_files::attachment_options(&format, record_path.as_deref())?;
        let connection = Connection::new(&endpoint)?;
        let connection = if let Some(token) = storage_options.token {
            connection.with_header("authorization", &format!("Bearer {token}"))?
        } else {
            connection
        };
        let archive_endpoint = unsafe { optional_string(reader_options.archive_endpoint) }?
            .unwrap_or_else(|| "https://codeload.github.com".into());
        let files =
            OpenStaxService::with_connections(connection, Connection::new(&archive_endpoint)?)
                .book(&locator, &revision)
                .await?;
        crate::service_files::open(files, file_name, attachment_options, scan_options).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
