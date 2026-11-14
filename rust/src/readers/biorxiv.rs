use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig, optional_string},
};
use open_arrow_access::service::biorxiv::{BiorxivService, Server};
use open_arrow_access::service::common::Connection;
use std::ffi::c_char;

#[repr(C)]
pub struct BiorxivOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub until: *const c_char,
    pub server: *const c_char,
    pub cursor: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_biorxiv_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &BiorxivOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let file_name = unsafe {
            optional_string(reader_options.file_name)?
                .filter(|s| !s.is_empty())
                .ok_or("file_name is required")?
        };
        let format =
            unsafe { optional_string(reader_options.format)?.unwrap_or_else(|| "binary".into()) };
        let record_path = unsafe { optional_string(reader_options.record_path)? };
        let endpoint = unsafe {
            optional_string(reader_options.endpoint)?
                .unwrap_or_else(|| "https://api.biorxiv.org".into())
        };
        let until = unsafe {
            optional_string(reader_options.until)?
                .filter(|s| !s.is_empty())
                .ok_or("until is required")?
        };
        let server =
            unsafe { optional_string(reader_options.server)?.unwrap_or_else(|| "biorxiv".into()) };
        let cursor =
            unsafe { optional_string(reader_options.cursor)?.unwrap_or_else(|| "0".into()) };
        crate::service_files::validate_format(&format, record_path.as_deref())?;
        let server = match server.as_str() {
            "biorxiv" => Server::Biorxiv,
            "medrxiv" => Server::Medrxiv,
            _ => return Err("server must be biorxiv or medrxiv".into()),
        };
        let connection = Connection::new(&endpoint)?;
        let connection = if let Some(token) = storage_options.token {
            connection.with_header("authorization", &format!("Bearer {token}"))?
        } else {
            connection
        };
        let service = BiorxivService::with_connection(connection);
        let files = service.jats(
            service
                .details(server, &locator, &until, cursor.parse()?)
                .await?,
        )?;
        crate::service_files::open(files, file_name, &format, record_path, scan_options).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
