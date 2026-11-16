use crate::{error::Result, options::StorageConfig, storage, utils, validation};
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{Sink, SinkExt, WriteOptions, Writer};
use open_arrow_sink_lance::{LanceDatasetSink, Storage};
use open_arrow_sink_layer::DownloadBlobLayer;
use std::collections::HashSet;

pub struct DuckportSink {
    writer: Option<Box<dyn Writer>>,
    pub(crate) schema: SchemaRef,
}
impl DuckportSink {
    pub(crate) async fn new(
        path: &str,
        schema: SchemaRef,
        mode: &str,
        columns: HashSet<String>,
        config: &StorageConfig,
    ) -> Result<Self> {
        let mode = utils::lance_mode(mode)?;
        let url = utils::lance_url(path)?;
        let (operator, _) = storage::operator(url.as_str(), config)?;
        // Match the resolver's local parent or S3 bucket-root namespace.
        let mut root = url.join("..")?;
        if url.scheme() == "s3" {
            root.set_path("/");
        }
        let sink = LanceDatasetSink::new(Storage::new(operator, root)?)
            .layer(DownloadBlobLayer::new(columns));
        let writer = sink
            .open(url, schema.clone(), WriteOptions::default().with_mode(mode))
            .await?;
        Ok(Self {
            writer: Some(writer),
            schema,
        })
    }
    pub(crate) async fn write(&mut self, batch: RecordBatch) -> Result<()> {
        let writer = validation::required(self.writer.as_mut(), "Lance writer is closed")?;
        let result = writer.write(batch).await;
        if result.is_err()
            && let Some(writer) = self.writer.take()
        {
            let _ = writer.abort().await;
        }
        Ok(result?)
    }
    pub(crate) async fn commit(&mut self) -> Result<()> {
        let writer = validation::required(self.writer.take(), "Lance writer is closed")?;
        Ok(writer.commit().await?)
    }
}
