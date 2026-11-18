use crate::utils::optional_string;
use open_arrow_access::service::federal_register::FederalRegisterService;
use std::ffi::c_char;

#[repr(C)]
pub struct FederalRegisterOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_federal_register_open, reader_options: FederalRegisterOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe {
            crate::service_files::SelectedAttachment::read(
                reader_options.file_name,
                reader_options.format,
                reader_options.record_path,
            )
        }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://www.federalregister.gov".into());
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::Bearer,
        )?;
        let files = FederalRegisterService::with_connection(connection)
            .document(&locator)
            .await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
