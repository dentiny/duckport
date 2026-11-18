use crate::source::files::open as open_file_source;
use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
};
use open_arrow_access::{
    OuterCompression,
    file_format::csv::{CsvDialect, CsvFileFormat, CsvHeader},
};
use std::ffi::c_char;

#[repr(C)]
pub struct CsvOptions {
    /// -1 infers the header; 0 disables it; 1 enables it.
    pub header: i32,
    pub delimiter: u8,
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_csv_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &CsvOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        let header = match reader_options.header {
            -1 => CsvHeader::Auto,
            value => CsvHeader::from(crate::utils::boolean("header", value)?),
        };

        let format = CsvFileFormat::infer().with_dialect(CsvDialect {
            header,
            delimiter: reader_options.delimiter,
            ..Default::default()
        });
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
