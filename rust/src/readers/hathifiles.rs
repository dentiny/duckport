use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::hathifiles::HathifilesService;
use std::ffi::c_char;

#[repr(C)]
pub struct HathifilesOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

impl_attachment_selection!(HathifilesOptions);

crate::ffi::reader::export_reader!(duckport_hathifiles_open, reader_options: HathifilesOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://www.hathitrust.org".into());
        crate::validation::validate_hathifiles_locator(&locator)?;
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::Bearer,
        )?;
        let files = HathifilesService::with_connection(connection)
            .catalog()
            .await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
