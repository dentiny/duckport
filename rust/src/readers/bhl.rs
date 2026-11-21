use crate::utils::optional_string;
use open_arrow_access::service::bhl::{BhlOpenData, open_data_bucket_operator};
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
        let _ = (locator, storage_options, scan_options, reader_options);
        Err("BHL API access was removed in open-arrow-access 0.17.0; use duckport_scan_bhl_open_data with explicit AWS object keys".into())
    }
);
