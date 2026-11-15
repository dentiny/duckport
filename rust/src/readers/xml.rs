use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, optional_string},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    OuterCompression, ReadRequest, file_accessor, file_format::xml::XmlFormat,
};
use std::ffi::c_char;

#[repr(C)]
pub struct XmlOptions {
    pub record_path: *const c_char,
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_xml_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &XmlOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        let record_path = unsafe { optional_string(reader_options.record_path) }?
            .ok_or("record_path is required")?;

        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let format = XmlFormat::new(record_path);
        AsyncSource::new(
            file_accessor(operator, format, OuterCompression::Auto),
            ReadRequest::new(path).with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
