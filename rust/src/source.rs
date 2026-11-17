use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{Accessor, BoundSource, FileSource, RecordReader};

pub struct DuckportSource(
    BoundSource,
    Option<std::sync::Arc<crate::readers::atif::MediaResolver>>,
);

impl DuckportSource {
    pub async fn new<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: Accessor<Reader = dyn RecordReader> + 'static,
        A::Request: Clone + Sync + 'static,
    {
        Ok(Self(BoundSource::bind(accessor, request).await?, None))
    }

    pub fn schema(&self) -> SchemaRef {
        match &self.1 {
            Some(_) => crate::readers::atif::embedded_schema(&self.0.schema()),
            None => self.0.schema(),
        }
    }

    pub async fn reader(&self) -> Result<Box<dyn RecordReader>> {
        let reader = self.0.reader().await?;
        if let Some(resolver) = &self.1 {
            let mut metadata = reader.metadata().clone();
            metadata.schema = self.schema();
            Ok(Box::new(AtifReader {
                inner: reader,
                resolver: resolver.clone(),
                metadata,
                terminal: false,
            }))
        } else {
            Ok(reader)
        }
    }
}

impl From<BoundSource> for DuckportSource {
    fn from(source: BoundSource) -> Self {
        Self(source, None)
    }
}

impl From<FileSource> for DuckportSource {
    fn from(source: FileSource) -> Self {
        Self(source.into_bound_source(), None)
    }
}

impl DuckportSource {
    pub(crate) fn with_atif(mut self, resolver: crate::readers::atif::MediaResolver) -> Self {
        self.1 = Some(std::sync::Arc::new(resolver));
        self
    }
}

struct AtifReader {
    terminal: bool,
    inner: Box<dyn RecordReader>,
    resolver: std::sync::Arc<crate::readers::atif::MediaResolver>,
    metadata: open_arrow_access::RecordMetadata,
}

#[open_arrow_access::async_trait]
impl open_arrow_access::Reader for AtifReader {
    type Item = arrow_array::RecordBatch;
    type Metadata = open_arrow_access::RecordMetadata;
    type Capability = open_arrow_access::RecordCapability;
    fn capability(&self) -> Self::Capability {
        self.inner.capability()
    }
    fn metadata(&self) -> &Self::Metadata {
        &self.metadata
    }
    async fn next(&mut self) -> open_arrow_access::Result<Option<Self::Item>> {
        if self.terminal {
            return Ok(None);
        }
        let result = async {
            match self.inner.next().await? {
                None => Ok(None),
                Some(batch) => self
                    .resolver
                    .resolve(batch, self.metadata.schema.clone())
                    .await
                    .map(Some)
                    .map_err(|error| {
                        open_arrow_access::Error::new(
                            open_arrow_access::ErrorKind::Decode,
                            "ATIF media",
                            error.to_string(),
                        )
                    }),
            }
        }
        .await;
        self.terminal = !matches!(&result, Ok(Some(_)));
        result
    }
}
