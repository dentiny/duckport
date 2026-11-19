//! Load a persisted snapshot with its own scoped DuckDB storage configuration.
use crate::{error::Result, options::StorageConfig, storage};
use arrow_select::concat::concat_batches;
use open_arrow_access::{
    Accessor, ReadRequest, file_format::parquet::ParquetAccessor,
    service::arxiv::ArxivMetadataIndex, storage::OpenDalAccessor,
};
use std::sync::Arc;

pub(super) async fn load(
    uri: &str,
    id_column: &str,
    config: &StorageConfig,
) -> Result<Arc<ArxivMetadataIndex>> {
    let (operator, path) = storage::operator(uri, config)?;
    let accessor = ParquetAccessor::new(OpenDalAccessor::new(operator));
    let mut reader = accessor
        .open(ReadRequest::new(path).with_byte_options(config.policy.byte_options()?))
        .await?;
    let schema = reader.metadata().schema.clone();
    let mut batches = Vec::new();
    while let Some(batch) = reader.next().await? {
        batches.push(batch);
    }
    let batch = concat_batches(&schema, &batches)?;
    Ok(Arc::new(ArxivMetadataIndex::try_new(batch, id_column)?))
}
