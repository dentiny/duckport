use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig, optional_string},
};
use open_arrow_access::service::common::Connection;
use open_arrow_access::service::hathifiles::HathifilesService;
use std::ffi::c_char;

#[repr(C)]
pub struct HathifilesOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_hathifiles_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &HathifilesOptions,
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
            .unwrap_or_else(|| "https://www.hathitrust.org".into());
        let attachment_options =
            crate::service_files::attachment_options(&format, record_path.as_deref())?;
        if locator != "catalog" {
            return Err("Hathifiles locator must be catalog".into());
        }
        let connection = Connection::new(&endpoint)?;
        let connection = if let Some(token) = storage_options.token {
            connection.with_header("authorization", &format!("Bearer {token}"))?
        } else {
            connection
        };
        let files = HathifilesService::with_connection(connection)
            .catalog()
            .await?;
        crate::service_files::open(files, file_name, attachment_options, scan_options).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
