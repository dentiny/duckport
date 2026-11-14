use crate::{
    ffi::{DuckportSource, open_reader},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    AccessorExt, ReadRequest,
    decompression::AutoDecompressionLayer,
    file_format::{FileAccessor, atif::AtifFormat},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

/// # Safety
/// Strings are NUL-terminated UTF-8; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_atif_jsonl_open(
    locator: *const c_char,
    options: *const c_char,
    batch_size: usize,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_reader(
            locator,
            options,
            batch_size,
            output,
            |locator, mut options, read_options| async move {
                let config = serde_json::from_str(
                    &options
                        .remove("storage_options")
                        .unwrap_or_else(|| "{}".into()),
                )?;
                let (operator, path) = storage::operator(&locator, config)?;
                let storage = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
                let format = AtifFormat::JsonLines;
                AsyncSource::new(
                    FileAccessor::from_format(storage, format),
                    ReadRequest::new(path).with_options(read_options),
                )
                .await
            },
        )
    }
}
