use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::zenodo::ZenodoService;
use std::ffi::c_char;

#[repr(C)]
pub struct ZenodoOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

impl_attachment_selection!(ZenodoOptions);

crate::ffi::reader::export_reader!(duckport_zenodo_open, reader_options: ZenodoOptions;
    |locator, storage_options, scan_options| async move {
        crate::provider_auth::reject_http_headers(storage_options.security.http()?, "Zenodo")?;
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://zenodo.org".into());
        let mut service = ZenodoService::with_endpoint(&endpoint)?
            .with_http_config(storage_options.policy.config()?)?;
        if let Some(token) = storage_options
            .security
            .http()?
            .and_then(|credentials| credentials.token())
        {
            service = service.with_token(token)?;
        }
        let files = service.record(locator.parse()?).await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
