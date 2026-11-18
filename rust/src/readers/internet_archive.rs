use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    utils::optional_string,
};
use open_arrow_access::service::internet_archive::InternetArchiveService;
use std::ffi::c_char;

#[repr(C)]
pub struct InternetArchiveOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_internet_archive_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &InternetArchiveOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let attachment = unsafe {
            crate::service_files::SelectedAttachment::read(
                reader_options.file_name,
                reader_options.format,
                reader_options.record_path,
            )
        }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://archive.org".into());
        let service = InternetArchiveService::with_endpoint(&endpoint)?
            .with_http_config(storage_options.policy.config()?)?;
        let service = if let Some((key, secret)) =
            crate::utils::internet_archive_credentials(&storage_options)?
        {
            service.with_credentials(key, secret)?
        } else {
            service
        };
        let files = service.item(&locator).await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
