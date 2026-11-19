use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::common::Connection;
use open_arrow_access::service::openstax::OpenStaxService;
use std::ffi::c_char;

#[repr(C)]
pub struct OpenstaxOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub revision: *const c_char,
    pub archive_endpoint: *const c_char,
}

impl_attachment_selection!(OpenstaxOptions);

crate::ffi::reader::export_reader!(duckport_openstax_open, reader_options: OpenstaxOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.github.com".into());
        let revision = crate::validation::nonempty(
            unsafe { optional_string(reader_options.revision) }?,
            "revision is required",
        )?;
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::Bearer,
        )?;
        let archive_endpoint = unsafe { optional_string(reader_options.archive_endpoint) }?
            .unwrap_or_else(|| "https://codeload.github.com".into());
        let files = OpenStaxService::with_connections(
            connection,
            Connection::new(&archive_endpoint)?
                .with_http_config(storage_options.policy.config()?)?,
        )
        .book(&locator, &revision)
        .await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
