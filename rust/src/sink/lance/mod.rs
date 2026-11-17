//! Lance dataset setup and backend-specific tests.
use super::DuckportSink;
use crate::{error::Result, options::StorageConfig, storage, utils};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{BlobHandleConfig, Sink, SinkExt, WriteOptions};
use open_arrow_sink_lance::LanceDatasetSink;
use open_arrow_sink_layer::DownloadBlobLayer;
use std::collections::HashSet;

mod ffi;
#[cfg(test)]
mod tests;

/// Byte sizes passed across the C ABI. -1 leaves a backend default unchanged.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DuckportLanceWriteOptions {
    pub blob_inline_size_threshold: i64,
    pub blob_dedicated_size_threshold: i64,
    pub target_file_size: i64,
}

impl Default for DuckportLanceWriteOptions {
    fn default() -> Self {
        Self {
            blob_inline_size_threshold: -1,
            blob_dedicated_size_threshold: -1,
            target_file_size: -1,
        }
    }
}

impl DuckportLanceWriteOptions {
    fn read(&self) -> Result<WriteOptions> {
        fn size(name: &str, value: i64, allow_zero: bool) -> Result<Option<usize>> {
            if value == -1 {
                return Ok(None);
            }
            if value < 0 || (!allow_zero && value == 0) {
                return Err(format!("{name} has an invalid byte size").into());
            }
            Ok(Some(
                usize::try_from(value).map_err(|_| format!("{name} exceeds platform size"))?,
            ))
        }
        let inline = size(
            "BLOB_INLINE_SIZE_THRESHOLD",
            self.blob_inline_size_threshold,
            true,
        )?;
        let dedicated = size(
            "BLOB_DEDICATED_SIZE_THRESHOLD",
            self.blob_dedicated_size_threshold,
            false,
        )?;
        let mut options = WriteOptions::default();
        options.target_file_size = size("TARGET_FILE_SIZE", self.target_file_size, false)?;
        if inline.is_some() || dedicated.is_some() {
            options.blob_handle = Some(BlobHandleConfig {
                inline_size_threshold: inline,
                dedicated_size_threshold: dedicated.and_then(std::num::NonZeroUsize::new),
                ..Default::default()
            });
        }
        Ok(options)
    }
}

pub(crate) async fn open(
    path: &str,
    schema: SchemaRef,
    mode: &str,
    columns: HashSet<String>,
    config: &StorageConfig,
    options: &DuckportLanceWriteOptions,
) -> Result<DuckportSink> {
    let options = options.read()?.with_mode(utils::sink_mode(mode)?);
    let destination = storage::sink_storage(path, config)?;
    let sink = LanceDatasetSink::new(destination.storage).layer(DownloadBlobLayer::new(columns));
    let writer = sink.open(destination.url, schema.clone(), options).await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
        output_schema: None,
    })
}
