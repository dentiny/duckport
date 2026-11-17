//! Sink backends and the shared COPY writer lifecycle.
mod lance;
mod parquet;

use crate::{error::Result, validation};
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use open_arrow_sink_core::Writer;

pub struct DuckportSink {
    writer: Option<Box<dyn Writer>>,
    pub(crate) schema: SchemaRef,
}
impl DuckportSink {
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
