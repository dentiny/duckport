use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::biorxiv::BiorxivService;
use std::ffi::c_char;

#[repr(C)]
pub struct BiorxivOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub until: *const c_char,
    pub server: *const c_char,
    pub cursor: *const c_char,
}

impl_attachment_selection!(BiorxivOptions);

crate::ffi::reader::export_reader!(duckport_biorxiv_open, reader_options: BiorxivOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.biorxiv.org".into());
        let until = crate::validation::nonempty(
            unsafe { optional_string(reader_options.until) }?,
            "until is required",
        )?;
        let server =
            unsafe { optional_string(reader_options.server) }?.unwrap_or_else(|| "biorxiv".into());
        let cursor =
            unsafe { optional_string(reader_options.cursor) }?.unwrap_or_else(|| "0".into());
        let server = crate::utils::biorxiv_server(&server)?;
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::Bearer,
        )?;
        let service = BiorxivService::with_connection(connection);
        let files = service.jats(
            service
                .details(server, &locator, &until, cursor.parse()?)
                .await?,
        )?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
