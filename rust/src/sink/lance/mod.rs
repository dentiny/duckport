//! Lance dataset setup and backend-specific tests.
use super::{DuckportSink, open_writer};
use crate::{error::Result, options::StorageConfig, storage, utils, validation};
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
    let mut options = options.with_mode(utils::sink_mode(mode)?);
    let metrics = open_arrow_sink_core::IoMetrics::default();
    options.io_metrics = Some(metrics.clone().into());
    let paths = columns
        .iter()
        .map(|name| utils::resolve_column_path(&schema, name, "BLOB_COLUMNS"))
        .collect::<Result<Vec<_>>>()?;
    let roots = paths.iter().map(|path| path[0].clone()).collect();
    validation::validate_blob_indexes(&options.indexes, &roots)?;
    let columns = paths.into_iter().map(|path| path.join(".")).collect();
    let destination = storage::sink_storage(path, config)?;
    let sink = LanceDatasetSink::new(destination.storage).layer(DownloadBlobLayer::new(columns));
    let writer = open_writer(sink, destination.url, schema.clone(), options, writer_count).await?;
    Ok(DuckportSink {
        metrics,
        writer: Some(writer),
        schema,
        output_schema: None,
    })
}
