use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, optional_string},
    source::{OuterCompression, open_file_source},
};
use open_arrow_access::file_format::xml::XmlFormat;
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
    storage_options: *const DuckportStorageOptions,
    scan_options: *const DuckportScanOptions,
    reader_options: *const XmlOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_reader(
            locator,
            storage_options,
            scan_options,
            output,
            |locator, storage_options, scan_options| async move {
                let reader_options = reader_options.as_ref().ok_or("null reader options")?;
                let record_path = optional_string(reader_options.record_path)?
                    .ok_or("record_path is required")?;

                open_file_source(
                    &locator,
                    &storage_options,
                    scan_options,
                    XmlFormat::new(record_path),
                    OuterCompression::Auto,
                )
                .await
            },
        )
    }
}
