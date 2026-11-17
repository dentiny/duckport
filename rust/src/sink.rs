use crate::{error::Result, options::StorageConfig, storage, utils, validation};
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{Sink, SinkExt, WriteOptions, Writer};
use open_arrow_sink_lance::{LanceDatasetSink, Storage};
use open_arrow_sink_layer::DownloadBlobLayer;
use std::collections::HashSet;

pub struct DuckportSink {
    pub(crate) writer: Option<Box<dyn Writer>>,
    pub(crate) schema: SchemaRef,
    pub(crate) output_schema: Option<SchemaRef>,
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
            output_schema: None,
        })
    }
    pub(crate) async fn write(&mut self, batch: RecordBatch) -> Result<()> {
        let writer = validation::required(self.writer.as_mut(), "sink writer is closed")?;
        let result = async {
            if batch.schema() != self.schema {
                return Err("sink batch schema does not match the opened schema".into());
            }
            let batch = if let Some(schema) = &self.output_schema {
                let columns = batch
                    .columns()
                    .iter()
                    .zip(schema.fields())
                    .map(|(array, field)| arrow_cast::cast(array, field.data_type()))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                RecordBatch::try_new(schema.clone(), columns)?
            } else {
                batch
            };
            writer.write(batch).await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        }
        .await;
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
