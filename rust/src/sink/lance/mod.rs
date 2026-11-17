//! Lance dataset setup and backend-specific tests.
use super::DuckportSink;
use crate::{error::Result, options::StorageConfig, storage, utils};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{Sink, SinkExt, WriteOptions};
use open_arrow_sink_lance::{LanceDatasetSink, Storage};
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
) -> Result<DuckportSink> {
    let mode = utils::lance_mode(mode)?;
    let url = utils::lance_url(path)?;
    let (operator, _) = storage::operator(url.as_str(), config)?;
    // Match the resolver's local parent or S3 bucket-root namespace.
    let mut root = url.join("..")?;
    if url.scheme() == "s3" {
        root.set_path("/");
    }
    let sink =
        LanceDatasetSink::new(Storage::new(operator, root)?).layer(DownloadBlobLayer::new(columns));
    let writer = sink
        .open(url, schema.clone(), WriteOptions::default().with_mode(mode))
        .await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
    })
}
