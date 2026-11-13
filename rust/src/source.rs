use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{
    Accessor, ReadOptions, ReadRequest, RecordReader, file_format::parquet::ParquetAccessor,
    storage::OpenDalAccessor,
};
use std::{num::NonZeroUsize, sync::Mutex};

pub struct DuckportSource {
    schema: SchemaRef,
    accessor: ParquetAccessor<OpenDalAccessor>,
    request: ReadRequest,
    bound: Mutex<Option<Box<dyn RecordReader>>>,
}
impl DuckportSource {
    pub async fn new(
        operator: open_arrow_access::storage::opendal::Operator,
        path: String,
        batch_size: NonZeroUsize,
    ) -> Result<Self> {
        let accessor = ParquetAccessor::new(OpenDalAccessor::new(operator));
        let request = ReadRequest::new(path).with_options(ReadOptions {
            batch_size,
            ..Default::default()
        });
        let reader = accessor.open(request.clone()).await?;
        let schema = reader.metadata().schema.clone();
        Ok(Self {
            schema,
            accessor,
            request,
            bound: Mutex::new(Some(reader)),
        })
    }
    pub fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    pub async fn reader(&self) -> Result<Box<dyn RecordReader>> {
        let bound = self
            .bound
            .lock()
            .map_err(|_| "source lock poisoned")?
            .take();
        let reader = match bound {
            Some(reader) => reader,
            None => self.accessor.open(self.request.clone()).await?,
        };
        if reader.metadata().schema != self.schema {
            return Err("source schema changed; rebind the query".into());
        }
        Ok(reader)
    }
}
