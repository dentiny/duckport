use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    AccessorExt, ReadRequest, decompression::AutoDecompressionLayer,
    service::gutenberg::GutenbergAccessor, storage::OpenDalAccessor,
};
use std::ffi::c_char;

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_gutenberg_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_reader(
            locator,
            storage_options,
            scan_options,
            output,
            |locator, storage_options, scan_options| async move {
                let (operator, path) = storage::operator(&locator, &storage_options)?;
                let storage = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
                AsyncSource::new(
                    GutenbergAccessor::with_default_decoder(storage),
                    ReadRequest::new(path).with_options(scan_options),
                )
                .await
            },
        )
    }
}
