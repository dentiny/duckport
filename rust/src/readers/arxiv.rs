use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
    validation::optional_string,
};
use open_arrow_access::{
    service::arxiv::{ArxivReadRequest, ArxivService},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;
#[repr(C)]
pub struct ArxivOptions {
    pub kind: *const c_char,
    pub month: *const c_char,
    pub requester_pays: i32,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_arxiv_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &ArxivOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let kind =
            unsafe { optional_string(reader_options.kind) }?.unwrap_or_else(|| "source".into());
        let month = unsafe { optional_string(reader_options.month) }?;
        let kind = crate::validation::arxiv_kind(&kind)?;
        let month = month.map(|s| s.parse::<u16>()).transpose()?;
        let requester_pays =
            crate::validation::boolean("requester_pays", reader_options.requester_pays)?;
        let storage = OpenDalAccessor::new(crate::storage::directory_operator_with_requester_pays(
            &locator,
            &storage_options,
            requester_pays,
        )?);
        let records = ArxivService::new(storage).records();
        let mut request = ArxivReadRequest::new(kind).with_options(scan_options);
        if let Some(month) = month {
            request = request.with_month(month);
        }
        AsyncSource::new(records, request).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
