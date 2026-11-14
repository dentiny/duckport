use crate::{
    ffi::{DuckportSource, open_reader},
    options::{ScanOptions, StorageOptions},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    AccessorExt, ReadRequest,
    decompression::AutoDecompressionLayer,
    file_format::{
        FileAccessor,
        csv::{CsvDialect, CsvFileFormat, CsvInferenceOptions},
    },
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_tsv_open(
    locator: *const c_char,
    storage_options: *const StorageOptions,
    scan_options: *const ScanOptions,
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
                let format = CsvFileFormat::infer()
                    .with_dialect(CsvDialect {
                        delimiter: b'\t',
                        ..Default::default()
                    })
                    .with_inference_options(CsvInferenceOptions {
                        has_header: scan_options.header,
                        ..Default::default()
                    });
                AsyncSource::new(
                    FileAccessor::from_format(storage, format),
                    ReadRequest::new(path).with_options(scan_options.read_options),
                )
                .await
            },
        )
    }
}
