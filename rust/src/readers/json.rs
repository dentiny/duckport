use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    source::{OuterCompression, open_file_source},
};
use open_arrow_access::file_format::json::JsonFormat;
use std::ffi::c_char;

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_json_open(
    locator: *const c_char,
    storage_options: *const DuckportStorageOptions,
    scan_options: *const DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        open_file_source(
            &locator,
            &storage_options,
            scan_options,
            JsonFormat::infer(),
            OuterCompression::Auto,
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
