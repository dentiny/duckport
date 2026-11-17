mod media;
mod policy;
use crate::source::open_file_source;
use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    storage,
};
pub(crate) use media::MediaResolver;
use open_arrow_access::{OuterCompression, ReadRequest, file_format::atif::AtifFormat};
use policy::{SOURCE_TYPE, STORAGE_POLICY, StoragePolicy};
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
    let open = |locator: String, storage, scan| async move { open(&locator, storage, scan).await };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}

pub(crate) async fn open(
    locator: &str,
    storage_options: crate::options::StorageConfig,
    scan_options: open_arrow_access::ReadOptions,
) -> crate::error::Result<crate::source::DuckportSource> {
    let (operator, path) = storage::operator(locator, &storage_options)?;
    let source = open_file_source(
        operator,
        ReadRequest::new(path).with_options(scan_options),
        AtifFormat::Auto,
        OuterCompression::Auto,
    )
    .await?;
    let document = if storage::is_local(locator) {
        crate::utils::local_path(locator)?
            .to_string_lossy()
            .into_owned()
    } else {
        locator.to_owned()
    };
    Ok(source.with_atif(MediaResolver { document }))
}
