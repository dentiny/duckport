use crate::{
    error::Result,
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
    storage,
};
use open_arrow_access::{
    AccessorExt, ReadOptions,
    decompression::AutoDecompressionLayer,
    file_format::{FileAccessor, warc::WarcArrowFormat},
    service::{
        commoncrawl::{CommonCrawlSegmentAccessor, CommonCrawlSegmentRequest},
        warc::{CommonCrawlWarcAccessor, CommonCrawlWarcRequest},
    },
    storage::{OpenDalAccessor, opendal},
};
use std::ffi::c_char;

enum Scope {
    Dataset,
    Segment,
}

fn create_location(locator: &str, options: &StorageConfig) -> Result<(opendal::Operator, String)> {
    if !storage::is_local(locator) {
        return storage::operator(locator, options);
    }
    let (root, key) = crate::utils::commoncrawl_location(locator)?;
    Ok((
        storage::observe(storage::create_local_directory_operator(&root)?, options),
        key,
    ))
}

async fn open_commoncrawl(
    locator: String,
    options: StorageConfig,
    scan: ReadOptions,
    scope: Scope,
) -> Result<AsyncSource> {
    let (operator, key) = create_location(&locator, &options)?;
    // Selection uses canonical inventory keys; the operator owns the physical location.
    let selection = format!("https://data.commoncrawl.org/{key}");
    let manifests = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
    let files = FileAccessor::from_format(manifests.clone(), WarcArrowFormat);
    match scope {
        Scope::Dataset => {
            AsyncSource::new(
                CommonCrawlWarcAccessor::new(manifests, files),
                CommonCrawlWarcRequest::from_url(&selection)?.with_options(scan),
            )
            .await
        }
        Scope::Segment => {
            AsyncSource::new(
                CommonCrawlSegmentAccessor::new(manifests, files),
                CommonCrawlSegmentRequest::from_url(&selection)?.with_options(scan),
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
