use crate::source::files::open as open_file_source;
use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
};
use open_arrow_access::{OuterCompression, file_format::warc::WarcArrowFormat};
use std::ffi::c_char;

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_warc_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        let format = WarcArrowFormat;
        open_file_source(
            &locator,
            &storage_options,
            scan_options,
            format,
            OuterCompression::Auto,
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
