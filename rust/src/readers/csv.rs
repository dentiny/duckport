use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    source::{OuterCompression, open_file_source},
};
use open_arrow_access::file_format::csv::{CsvDialect, CsvFileFormat, CsvInferenceOptions};
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
    storage_options: *const DuckportStorageOptions,
    scan_options: *const DuckportScanOptions,
    reader_options: *const CsvOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        let reader_options = unsafe { reader_options.as_ref() }.ok_or("null reader options")?;
        let header = if reader_options.header == -1 {
            None
        } else {
            Some(crate::validation::boolean("header", reader_options.header)?)
        };

        let format = CsvFileFormat::infer()
            .with_dialect(CsvDialect {
                delimiter: reader_options.delimiter,
                ..Default::default()
            })
            .with_inference_options(CsvInferenceOptions {
                has_header: header,
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
