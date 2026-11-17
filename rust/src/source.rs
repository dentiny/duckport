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
    claimed: std::sync::atomic::AtomicBool,
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
            claimed: std::sync::atomic::AtomicBool::new(false),
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
        if !self
            .claimed
            .swap(true, std::sync::atomic::Ordering::Relaxed)
            && options.projection.is_some()
        {
            // A projected scan cannot reuse the initial unprojected reader.
            // Release it now so its storage buffers and final IO observations
            // do not remain held until the prepared statement is destroyed.
            drop(self.source.scan(self.options.clone()).await?);
        }
        Ok(self.source.scan(options).await?)
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
