use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{Accessor, BoundSource, FileSource, RecordReader};

pub struct DuckportSource(BoundSource);

impl DuckportSource {
    pub async fn new<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: Accessor<Reader = dyn RecordReader> + 'static,
        A::Request: Clone + Sync + 'static,
    {
        Ok(Self(BoundSource::bind(accessor, request).await?))
    }

    pub fn schema(&self) -> SchemaRef {
        self.0.schema()
    }

    pub async fn reader(&self) -> Result<Box<dyn RecordReader>> {
        Ok(self.0.reader().await?)
    }
}

impl From<FileSource> for DuckportSource {
    fn from(source: FileSource) -> Self {
        Self(source.into_bound_source())
    }
}
