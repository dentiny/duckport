use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    utils::optional_string,
};
use open_arrow_access::service::common::Connection;
use open_arrow_access::service::figshare::FigshareService;
use std::ffi::c_char;

#[repr(C)]
pub struct FigshareOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub version: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_figshare_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &FigshareOptions,
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
            .unwrap_or_else(|| "https://api.figshare.com".into());
        let version =
            unsafe { optional_string(reader_options.version) }?.unwrap_or_else(|| "".into());
        let attachment_options = crate::utils::attachment_options(&format, record_path.as_deref())?;
        let connection =
            Connection::new(&endpoint)?.with_http_config(storage_options.policy.config()?)?;
        let connection = crate::provider_auth::apply(
            connection,
            &storage_options.security,
            crate::provider_auth::HttpAuth::FigshareToken,
        )?;
        let files = FigshareService::with_connection(connection)
            .article(
                locator.parse()?,
                if version.is_empty() {
                    None
                } else {
                    Some(version.parse()?)
                },
            )
            .await?;
        crate::service_files::open(files, file_name, attachment_options, scan_options).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
