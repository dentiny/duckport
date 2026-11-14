use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig, optional_string},
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::{
    Accessor, ReadRequest, RecordCapability, RecordReader, async_trait,
    service::arxiv::{ArxivKind, ArxivReadRequest, ArxivRecords, ArxivService},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;
struct Records {
    inner: ArxivRecords<OpenDalAccessor>,
    kind: ArxivKind,
    month: Option<u16>,
}
#[async_trait]
impl Accessor for Records {
    type Request = ReadRequest;
    type Reader = dyn RecordReader;
    fn capability(&self) -> RecordCapability {
        self.inner.capability()
    }
    async fn open(&self, request: ReadRequest) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        let mut selection = ArxivReadRequest::new(self.kind).with_options(request.options);
        if let Some(month) = self.month {
            selection = selection.with_month(month);
        }
        self.inner.open(selection).await
    }
}

#[repr(C)]
pub struct ArxivOptions {
    pub kind: *const c_char,
    pub month: *const c_char,
    pub requester_pays: i32,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_arxiv_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &ArxivOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let kind =
            unsafe { optional_string(reader_options.kind)?.unwrap_or_else(|| "source".into()) };
        let month = unsafe { optional_string(reader_options.month)? };
        let kind = match kind.as_str() {
            "source" => ArxivKind::Source,
            "pdf" => ArxivKind::Pdf,
            _ => return Err("kind must be source or pdf".into()),
        };
        let month = month.map(|s| s.parse::<u16>()).transpose()?;
        let requester_pays =
            crate::validation::boolean("requester_pays", reader_options.requester_pays)?;
        let storage = OpenDalAccessor::new(crate::storage::directory_operator_with_requester_pays(
            &locator,
            &storage_options,
            requester_pays,
        )?);
        let records = ArxivService::new(storage).records();
        AsyncSource::new(
            Records {
                inner: records,
                kind,
                month,
            },
            ReadRequest::new(locator).with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
