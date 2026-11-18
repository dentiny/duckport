use crate::utils::optional_string;
use open_arrow_access::service::uspto::UsptoService;
use std::ffi::c_char;

#[repr(C)]
pub struct UsptoOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub offset: *const c_char,
    pub page_size: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_uspto_open, reader_options: UsptoOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe {
            crate::service_files::SelectedAttachment::read(
                reader_options.file_name,
                reader_options.format,
                reader_options.record_path,
            )
        }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.uspto.gov".into());
        let offset =
            unsafe { optional_string(reader_options.offset) }?.unwrap_or_else(|| "0".into());
        let page_size =
            unsafe { optional_string(reader_options.page_size) }?.unwrap_or_else(|| "100".into());
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::ApiKeyHeader("x-api-key"),
        )?;
        let files = UsptoService::with_connection(connection)
            .product(&locator, offset.parse()?, page_size.parse()?)
            .await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
