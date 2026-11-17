//! Backend modules and the shared fixed-schema COPY writer lifecycle.
mod iceberg;
mod lance;
mod parquet;

use crate::{error::Result, validation};
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{ParallelSink, Sink, SinkExt, WriteOptions, Writer};
use open_arrow_sink_layer::UnorderedWriteLayer;
use std::num::NonZeroUsize;

pub(crate) async fn open_writer<S: ParallelSink>(
    sink: S,
    request: S::Request,
    schema: SchemaRef,
    options: WriteOptions,
    writer_count: usize,
) -> Result<Box<dyn Writer>> {
    // DuckDB schedules ordered batches before they reach the single writer.
    if writer_count == 1 {
        return Ok(sink.open(request, schema, options).await?);
    }
    let count = validation::required(
        NonZeroUsize::new(writer_count),
        "writer count must be positive",
    )?;
    Ok(sink
        .layer(UnorderedWriteLayer::new(count))
        .open(request, schema, options)
        .await?)
}

pub struct DuckportSink {
    pub(crate) writer: Option<Box<dyn Writer>>,
    pub(crate) schema: SchemaRef,
    pub(crate) output_schema: Option<SchemaRef>,
}
impl DuckportSink {
    pub(crate) async fn write(&mut self, batch: RecordBatch) -> Result<()> {
        let writer = validation::required(self.writer.as_mut(), "sink writer is closed")?;
        let result = async {
            validation::validate_sink_batch_schema(&batch.schema(), &self.schema)?;
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
