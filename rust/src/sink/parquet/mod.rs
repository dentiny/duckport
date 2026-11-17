//! Parquet directory setup.
mod ffi;

use super::DuckportSink;
use crate::{error::Result, options::StorageConfig, storage, utils};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{Sink, WriteOptions};
use open_arrow_sink_parquet::ParquetDirectorySink;

pub(crate) async fn open(
    path: &str,
    schema: SchemaRef,
    mode: &str,
    config: &StorageConfig,
) -> Result<DuckportSink> {
    let mode = utils::sink_mode(mode)?;
    let (url, storage) = storage::sink_storage(path, config)?;
    let writer = ParquetDirectorySink::new(storage)
        .open(url, schema.clone(), WriteOptions::default().with_mode(mode))
        .await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
    })
}
