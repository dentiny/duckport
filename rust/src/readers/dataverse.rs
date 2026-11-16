use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    utils::optional_string,
};
use open_arrow_access::service::dataverse::DataverseService;
use std::ffi::c_char;

#[repr(C)]
pub struct DataverseOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub version: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_dataverse_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &DataverseOptions,
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
            .unwrap_or_else(|| "https://dataverse.harvard.edu".into());
        let version = crate::validation::nonempty(
            unsafe { optional_string(reader_options.version) }?,
            "version is required",
        )?;
        let attachment_options = crate::utils::attachment_options(&format, record_path.as_deref())?;
        let service = DataverseService::new(&endpoint)?;
        let service = if let Some(token) = storage_options.token {
            service.with_bearer_token(&token)?
        } else {
            service
        };
        let files = service.dataset(&locator, &version).await?;
        crate::service_files::open(files, file_name, attachment_options, scan_options).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
