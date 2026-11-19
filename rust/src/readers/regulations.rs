use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::regulations::RegulationsService;
use std::ffi::c_char;

#[repr(C)]
pub struct RegulationsOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub resource: *const c_char,
}

impl_attachment_selection!(RegulationsOptions);

crate::ffi::reader::export_reader!(duckport_regulations_open, reader_options: RegulationsOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.regulations.gov".into());
        let resource = unsafe { optional_string(reader_options.resource) }?
            .unwrap_or_else(|| "documents".into());
        let resource = crate::utils::regulations_resource(&resource)?;
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::ApiKeyHeader("x-api-key"),
        )?;
        let files = RegulationsService::with_connection(connection)
            .record(resource, &locator)
            .await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
