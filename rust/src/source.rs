pub(crate) mod files;
mod scan;
use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{
    Accessor, AccessorExt, ByteAccessor, OuterCompression, PreparedSource, ReadOptions,
    ReadOptionsProvider, ReadRequest, ReaderFactory, RecordReader,
    decompression::AutoDecompressionLayer,
    storage::{OpenDalAccessor, opendal},
};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub struct DuckportSource {
    source: Arc<PreparedSource>,
    capability: open_arrow_access::RecordCapability,
    planned: bool,
    read_concurrency: AtomicUsize,
    options: ReadOptions,
    media: Option<Arc<crate::readers::atif::MediaResolver>>,
    // PMC shard plans currently select PDF; override its URL only after parallel reads.
    content_url_source_column: Option<&'static str>,
}

impl DuckportSource {
    pub async fn new<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: Accessor<Reader = dyn RecordReader> + 'static,
        A::Request: Clone + Sync + ReadOptionsProvider + 'static,
    {
        let options = request.read_options().clone();
        let capability = accessor.capability();
        Ok(Self::prepared(
            PreparedSource::prepare(accessor, request).await?,
            options,
            capability,
            false,
        ))
    }

    fn prepared(
        source: PreparedSource,
        options: ReadOptions,
        capability: open_arrow_access::RecordCapability,
        planned: bool,
    ) -> Self {
        Self {
            source: Arc::new(source),
            options,
            capability,
            planned,
            read_concurrency: AtomicUsize::new(1),
            media: None,
            content_url_source_column: None,
        }
    }

    pub(crate) async fn plannable<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: open_arrow_access::ScanPlanner + 'static,
        A::Request: Clone + Sync + ReadOptionsProvider + 'static,
    {
        let options = request.read_options().clone();
        let capability = accessor.capability();
        Ok(Self::prepared(
            PreparedSource::prepare_plannable(accessor, request).await?,
            options,
            capability,
            true,
        ))
    }

    pub(crate) async fn with_statistics<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: open_arrow_access::ScanPlanner + open_arrow_access::StatisticsProvider + 'static,
        A::Request: Clone + Sync + ReadOptionsProvider + 'static,
    {
        let options = request.read_options().clone();
        let capability = accessor.capability();
        Ok(Self::prepared(
            PreparedSource::prepare_plannable_with_statistics(accessor, request).await?,
            options,
            capability,
            true,
        ))
    }

    pub(crate) fn with_content_url_source_column(mut self, field: &'static str) -> Self {
        self.content_url_source_column = Some(field);
        self
    }

    pub(crate) fn set_read_concurrency(&self, count: usize) {
        self.read_concurrency.store(count, Ordering::Relaxed);
    }

    pub(crate) async fn statistics(&self) -> Result<open_arrow_access::ScanStatistics> {
        Ok(self.source.statistics(self.options.clone()).await?)
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
        let concurrency = self.read_concurrency.load(Ordering::Relaxed);
        let reader = if let Some(field) = self.content_url_source_column {
            // Transform outside the parallel executor, retaining native shard partitions.
            // The upstream layer resolves projections and obtains the selected URL field.
            scan::PreparedScan(self.source.clone(), self.capability)
                .layer(open_arrow_access::layers::ParallelReadLayer::new(
                    NonZeroUsize::new(concurrency).unwrap(),
                    NonZeroUsize::MIN,
                ))
                .layer(
                    open_arrow_access::layers::SchemaEvolutionLayer::try_new(vec![
                        open_arrow_access::layers::ColumnEvolution::remove_column("content_url"),
                        open_arrow_access::layers::ColumnEvolution::add_column_sql(
                            "content_url",
                            field,
                        ),
                    ])?
                    .with_source_schema(self.source.schema()),
                )
                .open(scan::ScanRequest(options))
                .await?
        } else if self.planned && concurrency > 1 {
            scan::PreparedScan(self.source.clone(), self.capability)
                .layer(open_arrow_access::layers::ParallelReadLayer::new(
                    NonZeroUsize::new(concurrency).unwrap(),
                    NonZeroUsize::MIN,
                ))
                .open(scan::ScanRequest(options))
                .await?
        } else {
            self.source.scan(options).await?
        };
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
    open_arrow_access::file_format::FileAccessor<Arc<dyn ByteAccessor>, F>:
        open_arrow_access::ScanPlanner<Request = ReadRequest>,
{
    let storage =
        crate::metrics::ObservedBytes::new(OpenDalAccessor::new(operator), metrics.clone());
    let storage: Arc<dyn ByteAccessor> = match compression {
        OuterCompression::None => Arc::new(storage),
        OuterCompression::Auto => Arc::new(storage.layer(AutoDecompressionLayer::new())),
    };
    DuckportSource::plannable(
        open_arrow_access::file_format::FileAccessor::from_format(storage, format),
        request,
    )
    .await
}

impl DuckportSource {
    pub(crate) fn with_atif(mut self, resolver: crate::readers::atif::MediaResolver) -> Self {
        self.media = Some(Arc::new(resolver));
        self
    }
}
