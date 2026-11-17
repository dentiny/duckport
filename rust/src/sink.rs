use crate::{
    error::Result,
    options::{SinkFormat, StorageConfig},
    storage, utils, validation,
};
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{Sink, SinkExt, WriteOptions, Writer};
use open_arrow_sink_lance::LanceDatasetSink;
use open_arrow_sink_layer::DownloadBlobLayer;
use open_arrow_sink_parquet::ParquetDirectorySink;
use std::collections::HashSet;

pub struct DuckportSink {
    writer: Option<Box<dyn Writer>>,
    pub(crate) schema: SchemaRef,
}
impl DuckportSink {
    pub(crate) async fn new(
        format: SinkFormat,
        path: &str,
        schema: SchemaRef,
        mode: &str,
        columns: HashSet<String>,
        config: &StorageConfig,
    ) -> Result<Self> {
        let options = WriteOptions::default().with_mode(utils::sink_mode(mode)?);
        let (url, storage) = storage::sink_storage(path, config)?;
        let writer = match format {
            SinkFormat::Lance => {
                LanceDatasetSink::new(storage)
                    .layer(DownloadBlobLayer::new(columns))
                    .open(url, schema.clone(), options)
                    .await?
            }
            SinkFormat::Parquet => {
                ParquetDirectorySink::new(storage)
                    .layer(DownloadBlobLayer::new(columns))
                    .open(url, schema.clone(), options)
                    .await?
            }
        };
        Ok(Self {
            writer: Some(writer),
            schema,
        })
    }
    pub(crate) async fn write(&mut self, batch: RecordBatch) -> Result<()> {
        let writer = validation::required(self.writer.as_mut(), "sink writer is closed")?;
        let result = writer.write(batch).await;
        if result.is_err()
            && let Some(writer) = self.writer.take()
        {
            let _ = writer.abort().await;
        }
        Ok(result?)
    }
    pub(crate) async fn commit(&mut self) -> Result<()> {
        let writer = validation::required(self.writer.take(), "sink writer is closed")?;
        Ok(writer.commit().await?)
    }
}
