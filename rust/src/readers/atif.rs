use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    storage,
};
use open_arrow_access::{
    OuterCompression, ReadRequest, file_format::atif::AtifFormat, open_file_source,
};
use std::ffi::c_char;

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
    let open = |locator: String, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let source = open_file_source(
            operator,
            ReadRequest::new(path).with_options(scan_options),
            AtifFormat::Auto,
            OuterCompression::Auto,
        )
        .await?;
        let source = crate::source::DuckportSource::from(source);
        let schema = crate::atif::source_schema(source.schema().as_ref(), &locator)?;
        Ok(source.with_schema(schema))
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
