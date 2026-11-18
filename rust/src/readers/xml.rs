use crate::source::files::open as open_file_source;
use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    utils::optional_string,
};
use open_arrow_access::{OuterCompression, file_format::xml::XmlFormat};
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
        let record_path = crate::validation::required(
            unsafe { optional_string(reader_options.record_path) }?,
            "record_path is required",
        )?;

        let format = XmlFormat::new(record_path);
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
