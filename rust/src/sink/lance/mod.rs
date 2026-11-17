//! Lance dataset setup and backend-specific tests.
use super::{DuckportSink, open_writer};
use crate::{error::Result, options::StorageConfig, storage, utils};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{SinkExt, WriteOptions};
use open_arrow_sink_lance::LanceDatasetSink;
use open_arrow_sink_layer::DownloadBlobLayer;
use open_arrow_sink_storage::Storage;
use std::{collections::HashSet, sync::Arc};

mod download;
mod ffi;
#[cfg(test)]
mod tests;

pub(crate) async fn open(
    path: &str,
    schema: SchemaRef,
    mode: &str,
    mut columns: HashSet<String>,
    config: &StorageConfig,
    options: WriteOptions,
    writer_count: usize,
    sources: Vec<Storage>,
) -> Result<DuckportSink> {
    let options = options.with_mode(utils::sink_mode(mode)?);
    columns.retain(|name| {
        !schema
            .field_with_name(name)
            .is_ok_and(crate::atif::is_media)
    });
    let destination = storage::sink_storage(path, config)?;
    let sink = LanceDatasetSink::new(destination.storage)
        .with_blob_downloader(Arc::new(download::with_sources(sources)?))
        .layer(DownloadBlobLayer::new(columns))
        .layer(crate::atif::sink_layer(&schema, &options));
    let writer = open_writer(sink, destination.url, schema.clone(), options, writer_count).await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
        output_schema: None,
    })
}
