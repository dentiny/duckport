use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{
    Accessor, OuterCompression, PreparedSource, ReadOptions, ReadOptionsProvider, ReadRequest,
    ReaderFactory, RecordReader, file_accessor, storage::opendal,
};

pub struct DuckportSource {
    source: PreparedSource,
    options: ReadOptions,
    media: Option<std::sync::Arc<crate::readers::atif::MediaResolver>>,
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
            media: None,
        })
    }

    pub fn schema(&self) -> SchemaRef {
        if self.media.is_some() {
            crate::readers::atif::embedded_schema(&self.source.schema())
        } else {
            self.source.schema()
        }
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
        let reader = self.source.scan(options).await?;
        if let Some(resolver) = &self.media {
            let mut metadata = reader.metadata().clone();
            metadata.schema = crate::readers::atif::embedded_schema(&metadata.schema);
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

impl DuckportSource {
    pub(crate) fn with_atif(mut self, resolver: crate::readers::atif::MediaResolver) -> Self {
        self.media = Some(std::sync::Arc::new(resolver));
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
