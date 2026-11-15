use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
};
use open_arrow_access::service::oapen::Oapen;
use std::ffi::c_char;

pub type OapenOptions = crate::oai::OaiOptions;

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_oapen_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &OapenOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let selection = unsafe { reader_options.read() }?;
        crate::oai::open::<Oapen>(locator, storage_options, scan_options, selection).await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
