use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{Accessor, ReadRequest, RecordReader};
use std::sync::Mutex;

pub struct DuckportSource {
    schema: SchemaRef,
    accessor: Box<dyn Accessor<Request = ReadRequest, Reader = dyn RecordReader>>,
    request: ReadRequest,
    bound: Mutex<Option<Box<dyn RecordReader>>>,
}
impl DuckportSource {
    pub async fn new(
        accessor: impl Accessor<Request = ReadRequest, Reader = dyn RecordReader> + 'static,
        request: ReadRequest,
    ) -> Result<Self> {
        let reader = accessor.open(request.clone()).await?;
        let schema = reader.metadata().schema.clone();
        Ok(Self {
            schema,
            accessor: Box::new(accessor),
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
