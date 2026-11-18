use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    utils::optional_string,
};
use open_arrow_access::service::common::Connection;
use open_arrow_access::service::govinfo::GovInfoService;
use std::ffi::c_char;

#[repr(C)]
pub struct GovinfoOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_govinfo_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &GovinfoOptions,
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
            .unwrap_or_else(|| "https://www.govinfo.gov".into());
        let attachment_options = crate::utils::attachment_options(&format, record_path.as_deref())?;
        let connection =
            Connection::new(&endpoint)?.with_http_config(storage_options.policy.config()?)?;
        let connection = if let Some(token) = storage_options.token {
            connection.with_header("authorization", &format!("Bearer {token}"))?
        } else {
            connection
        };
        let files = GovInfoService::with_connections(
            connection,
            Connection::new("https://api.govinfo.gov")?
                .with_http_config(storage_options.policy.config()?)?,
        )
        .bulk_directory(&locator)
        .await?;
        crate::service_files::open(
            files,
            file_name,
            attachment_options,
            scan_options,
            storage_options.metrics.clone(),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
