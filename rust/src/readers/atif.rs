mod media;
use crate::source::open_file_source;
use crate::storage;
pub(crate) use media::MediaResolver;
use open_arrow_access::{OuterCompression, ReadRequest, file_format::atif::AtifFormat};
pub(crate) const STORAGE_POLICY: &str = "duckport:storage_policy";

crate::ffi::reader::export_reader!(duckport_atif_open;
    |locator, storage, scan| async move { open(&locator, storage, scan).await }
);

pub(crate) async fn open(
    locator: &str,
    storage_options: crate::options::StorageConfig,
    scan_options: crate::options::ScanOptions,
) -> crate::error::Result<crate::source::DuckportSource> {
    let (operator, path) = storage::operator(locator, &storage_options)?;
    let source = open_file_source(
        operator,
        scan_options.request(ReadRequest::new(path)),
        AtifFormat::Auto,
        OuterCompression::Auto,
        &storage_options.metrics,
    )
    .await?;
    let document = if storage::is_local(locator) {
        url::Url::from_file_path(crate::utils::local_path(locator)?)
            .map_err(|_| "invalid ATIF document path")?
    } else {
        url::Url::parse(locator)?
    };
    Ok(source.with_atif(MediaResolver { document }))
}
