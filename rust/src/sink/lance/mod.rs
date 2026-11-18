//! Lance dataset setup and backend-specific tests.
use super::{DuckportSink, open_writer};
use crate::{error::Result, options::StorageConfig, storage, utils, validation};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{SinkExt, WriteOptions};
use open_arrow_sink_lance::LanceDatasetSink;
use open_arrow_sink_layer::DownloadBlobLayer;
use std::collections::HashSet;

mod ffi;

pub(crate) async fn open(
    path: &str,
    schema: SchemaRef,
    mode: &str,
    columns: HashSet<String>,
    config: &StorageConfig,
    options: WriteOptions,
    writer_count: usize,
    secrets: Vec<crate::security::ScopedHttpCredentials>,
) -> Result<DuckportSink> {
    let options = options.with_mode(utils::sink_mode(mode)?);
    let paths = columns
        .iter()
        .map(|name| utils::resolve_column_path(&schema, name, "BLOB_COLUMNS"))
        .collect::<Result<Vec<_>>>()?;
    let roots = paths.iter().map(|path| path[0].clone()).collect();
    validation::validate_blob_indexes(&options.indexes, &roots)?;
    let columns = paths.into_iter().map(|path| path.join(".")).collect();
    let destination = storage::sink_storage(path, config)?;
    let downloader = crate::blob_auth::blob_downloader(secrets, config.policy)?;
    let sink = LanceDatasetSink::new(destination.storage)
        .with_blob_downloader(std::sync::Arc::new(downloader))
        .layer(DownloadBlobLayer::new(columns));
    let writer = open_writer(sink, destination.url, schema.clone(), options, writer_count).await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
        output_schema: None,
    })
}
