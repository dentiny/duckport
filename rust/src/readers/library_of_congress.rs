use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::common::Connection;
use open_arrow_access::service::library_of_congress::LibraryOfCongressService;
use std::ffi::c_char;

#[repr(C)]
pub struct LibraryOfCongressOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

impl_attachment_selection!(LibraryOfCongressOptions);

crate::ffi::reader::export_reader!(duckport_library_of_congress_open, reader_options: LibraryOfCongressOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://www.loc.gov".into());
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::Bearer,
        )?;
        let files = LibraryOfCongressService::with_connections(
            connection,
            Connection::new("https://tile.loc.gov")?
                .with_http_config(storage_options.policy.config()?)?,
        )
        .resource(&locator)
        .await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
