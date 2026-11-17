use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{Accessor, BoundSource, FileSource, RecordReader};

pub struct DuckportSource {
    source: BoundSource,
    schema: Option<SchemaRef>,
}

impl DuckportSource {
    pub async fn new<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: Accessor<Reader = dyn RecordReader> + 'static,
        A::Request: Clone + Sync + 'static,
    {
        Ok(Self::from(BoundSource::bind(accessor, request).await?))
    }

    pub fn with_schema(mut self, schema: SchemaRef) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn schema(&self) -> SchemaRef {
        self.schema.clone().unwrap_or_else(|| self.source.schema())
    }

    pub async fn reader(&self) -> Result<Box<dyn RecordReader>> {
        let reader = self.source.reader().await?;
        let Some(schema) = &self.schema else {
            return Ok(reader);
        };
        let schema = schema.clone();
        let capability = reader.capability();
        Ok(Box::new(open_arrow_access::map_record_reader(
            reader,
            schema.clone(),
            capability,
            move |batch| {
                crate::utils::apply_batch_schema(batch, schema.clone()).map_err(|error| {
                    open_arrow_access::Error::new(
                        open_arrow_access::ErrorKind::Decode,
                        "source_schema",
                        error.to_string(),
                    )
                })
            },
        )))
    }
}

impl From<BoundSource> for DuckportSource {
    fn from(source: BoundSource) -> Self {
        Self {
            source,
            schema: None,
        }
    }
}

impl From<FileSource> for DuckportSource {
    fn from(source: FileSource) -> Self {
        Self::from(source.into_bound_source())
    }
}
