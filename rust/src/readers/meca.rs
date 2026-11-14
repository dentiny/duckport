use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::{
    Accessor, ByteReadRequest, ReadRequest, RecordCapability, RecordReader, async_trait,
    decompression::zip::ZipLimits,
    service::meca::{MecaArchive, MecaReadRequest, MecaRecords},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;
struct Records(MecaRecords);
#[async_trait]
impl Accessor for Records {
    type Request = ReadRequest;
    type Reader = dyn RecordReader;
    fn capability(&self) -> RecordCapability {
        self.0.capability()
    }
    async fn open(&self, request: ReadRequest) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        self.0
            .open(MecaReadRequest::default().with_options(request.options))
            .await
    }
}

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
            Records(records),
            ReadRequest::new(locator).with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
