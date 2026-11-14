use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::{
    Accessor, ByteReadRequest,
    decompression::zip::ZipLimits,
    service::meca::{MecaArchive, MecaReadRequest},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;
/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_meca_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let (operator, path) = crate::storage::operator(&locator, &storage_options)?;
        let input = OpenDalAccessor::new(operator)
            .open(ByteReadRequest::new(path))
            .await?;
        let records = MecaArchive::open(input, ZipLimits::default())
            .await?
            .records();
        AsyncSource::new(
            records,
            MecaReadRequest::default().with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
