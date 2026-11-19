use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
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

impl_attachment_selection!(FigshareOptions);

crate::ffi::reader::export_reader!(duckport_figshare_open, reader_options: FigshareOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.figshare.com".into());
        let version =
            unsafe { optional_string(reader_options.version) }?.unwrap_or_else(|| "".into());
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
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
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
