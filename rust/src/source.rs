pub(crate) mod files;
use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{
    Accessor, AccessorExt, ByteAccessor, FactoryAccessor, OuterCompression, PreparedSource,
    ReadOptions, ReadOptionsProvider, ReadRequest, ReaderFactory, RecordReader,
    decompression::AutoDecompressionLayer,
    storage::{OpenDalAccessor, opendal},
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
        if let Some(resolver) = &self.media {
            resolver.schema(&self.source.schema())
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
        match &self.media {
            Some(resolver) => Ok(resolver.reader(reader)),
            None => Ok(reader),
        }
    }
}

pub(crate) async fn open_file_source<F>(
    operator: opendal::Operator,
    request: ReadRequest,
    format: F,
    compression: OuterCompression,
    metrics: &crate::metrics::ReadMetrics,
) -> Result<DuckportSource>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + 'static,
{
    let storage =
        crate::metrics::ObservedBytes::new(OpenDalAccessor::new(operator), metrics.clone());
    let storage: std::sync::Arc<dyn ByteAccessor> = match compression {
        OuterCompression::None => std::sync::Arc::new(storage),
        OuterCompression::Auto => std::sync::Arc::new(storage.layer(AutoDecompressionLayer::new())),
    };
    DuckportSource::new(FactoryAccessor::new(storage, format), request).await
}

impl DuckportSource {
    pub(crate) fn with_atif(mut self, resolver: crate::readers::atif::MediaResolver) -> Self {
        self.media = Some(std::sync::Arc::new(resolver));
        self
    }
}
