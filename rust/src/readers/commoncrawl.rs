use crate::{
    error::Result, options::StorageConfig, source::DuckportSource as AsyncSource, storage,
};
use open_arrow_access::{
    AccessorExt,
    decompression::AutoDecompressionLayer,
    file_format::{FileAccessor, warc::WarcArrowFormat},
    service::{
        commoncrawl::{CommonCrawlSegmentAccessor, CommonCrawlSegmentRequest},
        warc::{CommonCrawlWarcAccessor, CommonCrawlWarcRequest},
    },
    storage::{OpenDalAccessor, opendal},
};

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
    scan: crate::options::ScanOptions,
    scope: Scope,
) -> Result<AsyncSource> {
    let (operator, key) = create_location(&locator, &options)?;
    // Selection uses canonical inventory keys; the operator owns the physical location.
    let selection = format!("https://data.commoncrawl.org/{key}");
    let manifests = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
    let files = FileAccessor::from_format(manifests.clone(), WarcArrowFormat);
    match scope {
        Scope::Dataset => {
            AsyncSource::plannable(
                CommonCrawlWarcAccessor::new(manifests, files).with_file_planning(),
                scan.request(CommonCrawlWarcRequest::from_url(&selection)?),
            )
            .await
        }
        Scope::Segment => {
            AsyncSource::plannable(
                CommonCrawlSegmentAccessor::new(manifests, files).with_file_planning(),
                scan.request(CommonCrawlSegmentRequest::from_url(&selection)?),
            )
            .await
        }
    }
}

crate::ffi::reader::export_reader!(duckport_commoncrawl_open;
    |locator, options, scan| open_commoncrawl(locator, options, scan, Scope::Dataset)
);

crate::ffi::reader::export_reader!(duckport_commoncrawl_segment_open;
    |locator, options, scan| open_commoncrawl(locator, options, scan, Scope::Segment)
);
