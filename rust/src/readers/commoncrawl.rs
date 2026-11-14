use crate::{
    error::Result,
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    Accessor, AccessorExt, ReadOptions, ReadOptionsProvider, ReadRequest, RecordCapability,
    RecordReader, async_trait,
    decompression::AutoDecompressionLayer,
    file_format::{FileAccessor, warc::WarcArrowFormat},
    service::{
        commoncrawl::{CommonCrawlSegmentAccessor, CommonCrawlSegmentRequest},
        warc::{CommonCrawlWarcAccessor, CommonCrawlWarcRequest},
    },
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

struct CrawlSource<A: Accessor> {
    accessor: A,
    request: A::Request,
}

#[async_trait]
impl<A> Accessor for CrawlSource<A>
where
    A: Accessor<Reader = dyn RecordReader>,
    A::Request: Clone + ReadOptionsProvider + Sync,
{
    type Request = ReadRequest;
    type Reader = dyn RecordReader;

    fn capability(&self) -> RecordCapability {
        self.accessor.capability()
    }
    async fn open(&self, request: ReadRequest) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        let mut selected = self.request.clone();
        *selected.read_options_mut() = request.options;
        self.accessor.open(selected).await
    }
}

enum Scope {
    Dataset,
    Segment,
}

async fn open_commoncrawl(
    locator: String,
    options: StorageConfig,
    scan: ReadOptions,
    scope: Scope,
) -> Result<AsyncSource> {
    let (operator, _) = storage::operator(&locator, &options)?;
    let manifests = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
    let files = FileAccessor::from_format(manifests.clone(), WarcArrowFormat);
    let read = ReadRequest::new(locator.clone()).with_options(scan);
    match scope {
        Scope::Dataset => {
            let request = CommonCrawlWarcRequest::from_url(&locator)?;
            AsyncSource::new(
                CrawlSource {
                    accessor: CommonCrawlWarcAccessor::new(manifests, files),
                    request,
                },
                read,
            )
            .await
        }
        Scope::Segment => {
            let request = CommonCrawlSegmentRequest::from_url(&locator)?;
            AsyncSource::new(
                CrawlSource {
                    accessor: CommonCrawlSegmentAccessor::new(manifests, files),
                    request,
                },
                read,
            )
            .await
        }
    }
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_commoncrawl_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator, options, scan| open_commoncrawl(locator, options, scan, Scope::Dataset);
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_commoncrawl_segment_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator, options, scan| open_commoncrawl(locator, options, scan, Scope::Segment);
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
