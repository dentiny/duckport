use crate::{
    ffi::{DuckportSource, open_reader},
    options::{ScanOptions, StorageOptions},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    AccessorExt, ReadRequest,
    decompression::AutoDecompressionLayer,
    file_format::{FileAccessor, xml::XmlFormat},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

/// # Safety
/// Locator and option JSON are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_xml_open(
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
            |locator, storage_options, mut options, read_options| async move {
                let (operator, path) = storage::operator(&locator, storage_options)?;
                let storage = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
                let format = XmlFormat::new(
                    options
                        .remove("record_path")
                        .ok_or("record_path is required")?,
                );
                AsyncSource::new(
                    FileAccessor::from_format(storage, format),
                    ReadRequest::new(path).with_options(read_options),
                )
                .await
            },
        )
    }
}
