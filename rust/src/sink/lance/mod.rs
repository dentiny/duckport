//! Lance dataset setup and backend-specific tests.
use super::{DuckportSink, open_writer};
use crate::{error::Result, options::StorageConfig, storage, utils};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{SinkExt, WriteOptions};
use open_arrow_sink_lance::LanceDatasetSink;
use open_arrow_sink_layer::DownloadBlobLayer;
use std::collections::HashSet;

mod ffi;
#[cfg(test)]
mod tests;

pub(crate) async fn open(
    path: &str,
    schema: SchemaRef,
    mode: &str,
    columns: HashSet<String>,
    config: &StorageConfig,
    options: WriteOptions,
    writer_count: usize,
) -> Result<DuckportSink> {
    let options = options.with_mode(utils::sink_mode(mode)?);
    let destination = storage::sink_storage(path, config)?;
    let sink = LanceDatasetSink::new(destination.storage).layer(DownloadBlobLayer::new(columns));
    let writer = open_writer(sink, destination.url, schema.clone(), options, writer_count).await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
        output_schema: None,
    })
}
