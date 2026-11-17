use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    utils::optional_string,
};
use open_arrow_access::service::biorxiv::BiorxivService;
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
        let file_name = crate::validation::nonempty(
            unsafe { optional_string(reader_options.file_name) }?,
            "file_name is required",
        )?;
        let format =
            unsafe { optional_string(reader_options.format) }?.unwrap_or_else(|| "binary".into());
        let record_path = unsafe { optional_string(reader_options.record_path) }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.biorxiv.org".into());
        let until = crate::validation::nonempty(
            unsafe { optional_string(reader_options.until) }?,
            "until is required",
        )?;
        let server =
            unsafe { optional_string(reader_options.server) }?.unwrap_or_else(|| "biorxiv".into());
        let cursor =
            unsafe { optional_string(reader_options.cursor) }?.unwrap_or_else(|| "0".into());
        let attachment_options = crate::utils::attachment_options(&format, record_path.as_deref())?;
        let server = crate::utils::biorxiv_server(&server)?;
        let connection =
            Connection::new(&endpoint)?.with_http_config(storage_options.policy.config()?)?;
        let connection = if let Some(token) = storage_options.security.token {
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
        crate::service_files::open(files, file_name, attachment_options, scan_options).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
