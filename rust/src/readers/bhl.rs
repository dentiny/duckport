use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::bhl::BhlService;
use open_arrow_access::service::common::Connection;
use std::ffi::c_char;

#[repr(C)]
pub struct BhlOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

impl_attachment_selection!(BhlOptions);

crate::ffi::reader::export_reader!(duckport_bhl_open, reader_options: BhlOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://www.biodiversitylibrary.org".into());
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::ApiKeyQuery("apikey"),
        )?;
        let files = BhlService::with_services(
            connection,
            open_arrow_access::service::figshare::FigshareService::with_connection(
                Connection::new("https://api.figshare.com")?
                    .with_http_config(storage_options.policy.config()?)?,
            ),
        )
        .item(locator.parse()?)
        .await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
