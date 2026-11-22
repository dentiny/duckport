use crate::{metrics::ObservedBytes, utils::optional_string};
use open_arrow_access::{
    service::bhl::{BhlOpenData, BhlShardAccessor, BhlShardReadRequest, open_data_bucket_operator},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

#[repr(C)]
pub struct BhlOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
}

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
        let bytes = storage(&locator, &storage_options)?;
        let files = BhlOpenData::new(bytes).partition(keys.clone())?;
        crate::source::DuckportSource::plannable(
            open_arrow_access::service::FileSetAccessor::new(open_arrow_access::AttachmentAccessor::new(files, options)?).with_file_planning(),
            scan_options.request(open_arrow_access::service::FileSetRequest::new(keys)),
        ).await
    }
);

crate::ffi::reader::export_reader!(duckport_bhl_open, reader_options: BhlOptions;
    |locator, storage_options, scan_options| async move {
        let _ = (locator, storage_options, scan_options, reader_options);
        Err("BHL API access was removed in open-arrow-access 0.17.0; use duckport_scan_bhl_open_data with explicit AWS object keys".into())
    }
);

#[repr(C)]
pub struct BhlPagesOptions {
    pub prefix: *const c_char,
    pub metadata_db: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_bhl_pages_open, reader_options: BhlPagesOptions;
    |locator, storage_options, scan_options| async move {
        let prefix = crate::validation::nonempty(unsafe { optional_string(reader_options.prefix) }?, "prefix is required")?;
        let request = BhlShardReadRequest::new(prefix)?;
        let database = crate::validation::nonempty(unsafe { optional_string(reader_options.metadata_db) }?, "metadata_db is required")?;
        let bytes = storage(&locator, &storage_options)?;
        let source = BhlShardAccessor::from_database(bytes, database).await?;
        crate::source::DuckportSource::plannable(source, scan_options.request(request)).await
    }
);

fn storage(
    locator: &str,
    config: &crate::options::StorageConfig,
) -> crate::error::Result<ObservedBytes<OpenDalAccessor>> {
    let operator = if crate::validation::is_public_bhl_root(locator) {
        crate::storage::observe(
            config
                .policy
                .config()?
                .apply(open_data_bucket_operator()?)?,
            config,
        )
    } else {
        crate::storage::directory_operator(locator, config)?
    };
    Ok(ObservedBytes::new(
        OpenDalAccessor::new(operator),
        config.metrics.clone(),
    ))
}
