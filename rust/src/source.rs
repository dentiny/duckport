use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{
    Accessor, OuterCompression, PreparedSource, ReadOptions, ReadOptionsProvider, ReadRequest,
    ReaderFactory, RecordReader, file_accessor, storage::opendal,
};

pub struct DuckportSource {
    source: PreparedSource,
    options: ReadOptions,
}

impl DuckportSource {
    pub async fn new<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: Accessor<Reader = dyn RecordReader> + 'static,
        A::Request: Clone + Sync + ReadOptionsProvider + 'static,
    {
        let options = request.read_options().clone();
        Ok(Self {
            source: PreparedSource::prepare(accessor, request).await?,
            options,
        })
    }

    pub fn schema(&self) -> SchemaRef {
        self.source.schema()
    }

    pub async fn reader(&self, projection: Vec<String>) -> Result<Box<dyn RecordReader>> {
        let mut options = self.options.clone();
        // None reuses the reader opened at bind, avoiding another open for a full scan.
        // Other projections, including an empty list for COUNT(*), are passed through.
        options.projection = if projection.iter().map(String::as_str).eq(self
            .source
            .schema()
            .fields()
            .iter()
            .map(|field| field.name().as_str()))
        {
            None
        } else {
            Some(projection)
        };
        Ok(self.source.scan(options).await?)
    }
}

pub(crate) async fn open_file_source<F>(
    operator: opendal::Operator,
    request: ReadRequest,
    format: F,
    compression: OuterCompression,
) -> Result<DuckportSource>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + 'static,
{
    DuckportSource::new(file_accessor(operator, format, compression), request).await
}
