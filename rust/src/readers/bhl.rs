use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::service::bhl::{BhlOpenData, BhlService, open_data_bucket_operator};
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

#[repr(C)]
pub struct BhlOpenDataOptions {
    pub keys: *const *const c_char,
    pub key_count: usize,
    pub format: *const c_char,
    pub record_path: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_bhl_open_data_open, reader_options: BhlOpenDataOptions;
    |locator, storage_options, scan_options| async move {
        let keys = unsafe { crate::utils::ffi_slice(reader_options.keys, reader_options.key_count, crate::error::BHL_KEYS_NULL) }?
            .iter().map(|key| unsafe { crate::utils::string(*key) }).collect::<crate::error::Result<Vec<_>>>()?;
        let format = unsafe { optional_string(reader_options.format) }?;
        let record_path = unsafe { optional_string(reader_options.record_path) }?;
        let options = crate::utils::attachment_options(format.as_deref().unwrap_or("binary"), record_path.as_deref())?;
        let operator = if crate::validation::is_public_bhl_root(&locator) {
            crate::storage::observe(storage_options.policy.config()?.apply(open_data_bucket_operator()?)?, &storage_options)
        } else {
            crate::storage::directory_operator(&locator, &storage_options)?
        };
        let bytes = crate::metrics::ObservedBytes::new(open_arrow_access::storage::OpenDalAccessor::new(operator), storage_options.metrics.clone());
        let files = BhlOpenData::new(bytes).partition(keys.clone())?;
        crate::source::DuckportSource::plannable(
            open_arrow_access::service::FileSetAccessor::new(open_arrow_access::AttachmentAccessor::new(files, options)?).with_file_planning(),
            scan_options.request(open_arrow_access::service::FileSetRequest::new(keys)),
        ).await
    }
);

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
