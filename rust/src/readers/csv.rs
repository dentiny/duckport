use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    AccessorExt, ReadRequest,
    decompression::AutoDecompressionLayer,
    file_format::{
        FileAccessor,
        csv::{CsvDialect, CsvFileFormat, CsvHeader},
    },
    storage::OpenDalAccessor,
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
            value => CsvHeader::from(crate::validation::boolean("header", value)?),
        };

        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let storage = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
        let format = CsvFileFormat::infer().with_dialect(CsvDialect {
            header,
            delimiter: reader_options.delimiter,
            ..Default::default()
        });
        AsyncSource::new(
            FileAccessor::from_format(storage, format),
            ReadRequest::new(path).with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
