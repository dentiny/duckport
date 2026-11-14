use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    AccessorExt, ReadRequest,
    decompression::AutoDecompressionLayer,
    file_format::{FileAccessor, atif::AtifFormat},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

unsafe fn open_atif(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
    format: AtifFormat,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let storage = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
        AsyncSource::new(
            FileAccessor::from_format(storage, format),
            ReadRequest::new(path).with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_atif_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_atif(
            locator,
            storage_options,
            scan_options,
            output,
            AtifFormat::Json,
        )
    }
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_atif_jsonl_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_atif(
            locator,
            storage_options,
            scan_options,
            output,
            AtifFormat::JsonLines,
        )
    }
}
