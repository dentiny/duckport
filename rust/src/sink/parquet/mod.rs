//! Parquet directory setup.
mod ffi;

use super::{DuckportSink, open_writer};
use crate::{error::Result, options::StorageConfig, storage, utils};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::WriteOptions;
use open_arrow_sink_parquet::ParquetDirectorySink;

pub(crate) async fn open(
    path: &str,
    schema: SchemaRef,
    mode: &str,
    config: &StorageConfig,
    options: WriteOptions,
    writer_count: usize,
) -> Result<DuckportSink> {
    let mode = utils::sink_mode(mode)?;
    let destination = storage::sink_storage(path, config)?;
    let writer = open_writer(
        ParquetDirectorySink::new(destination.storage),
        destination.url,
        schema.clone(),
        options.with_mode(mode),
        writer_count,
    )
    .await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
        output_schema: None,
    })
}
