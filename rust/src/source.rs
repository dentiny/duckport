use crate::error::{Result, read_batch};
use arrow_array::{RecordBatch, RecordBatchReader};
use arrow_schema::{ArrowError, SchemaRef};
use open_arrow_access::{
    Accessor, ReadOptions, ReadRequest, RecordReader, file_format::parquet::ParquetAccessor,
    storage::OpenDalAccessor,
};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use tokio::runtime::Runtime;

pub struct DuckportSource {
    runtime: Arc<Runtime>,
    schema: SchemaRef,
    accessor: ParquetAccessor<OpenDalAccessor>,
    request: ReadRequest,
    bound: Mutex<Option<Box<dyn RecordReader>>>,
}
impl DuckportSource {
    pub fn new(
        operator: open_arrow_access::storage::opendal::Operator,
        path: String,
        batch_size: NonZeroUsize,
    ) -> Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        );
        let accessor = ParquetAccessor::new(OpenDalAccessor::new(operator));
        let request = ReadRequest::new(path).with_options(ReadOptions {
            batch_size,
            ..Default::default()
        });
        let reader = runtime.block_on(accessor.open(request.clone()))?;
        let schema = reader.metadata().schema.clone();
        Ok(Self {
            runtime,
            schema,
            accessor,
            request,
            bound: Mutex::new(Some(reader)),
        })
    }
    pub fn reader(&self) -> Result<BatchReader> {
        let reader = match self
            .bound
            .lock()
            .map_err(|_| "source lock poisoned")?
            .take()
        {
            Some(reader) => reader,
            None => self
                .runtime
                .block_on(self.accessor.open(self.request.clone()))?,
        };
        if reader.metadata().schema != self.schema {
            return Err("source schema changed; rebind the query".into());
        }
        Ok(BatchReader {
            reader: Some(reader),
            runtime: self.runtime.clone(),
            schema: self.schema.clone(),
            done: false,
        })
    }
}
impl Drop for DuckportSource {
    fn drop(&mut self) {
        let _guard = self.runtime.enter();
        if let Ok(reader) = self.bound.get_mut() {
            reader.take();
        }
    }
}
pub struct BatchReader {
    reader: Option<Box<dyn RecordReader>>,
    runtime: Arc<Runtime>,
    schema: SchemaRef,
    done: bool,
}
impl Iterator for BatchReader {
    type Item = std::result::Result<RecordBatch, ArrowError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match read_batch(|| self.runtime.block_on(self.reader.as_mut().unwrap().next())) {
            Ok(batch) => {
                self.done = batch.is_none();
                batch.map(Ok)
            }
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
}
impl RecordBatchReader for BatchReader {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}
impl Drop for BatchReader {
    fn drop(&mut self) {
        let _guard = self.runtime.enter();
        self.reader.take();
    }
}
