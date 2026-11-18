//! Compose independently configured readers through the upstream file set.
use super::*;
use crate::options::StorageConfig;
use open_arrow_access::{
    RecordCapability, RecordMetadata, ScanPlan, ScanPlanner, ScanStatistics, Statistic,
    StatisticsProvider, async_trait,
    dataset::file_set::{FileSetAccessor, FileSetRequest},
    file_format::FileAccessor,
};
use std::{collections::HashMap, sync::Arc};

struct Input {
    operator: opendal::Operator,
    path: String,
}
struct Files<F> {
    // Maps each selected file locator to its storage operator and operator-relative path.
    inputs: HashMap<String, Input>,
    format: F,
    compression: OuterCompression,
    metrics: crate::metrics::ReadMetrics,
}
impl<F: Clone> Files<F> {
    fn file(
        &self,
        request: &mut ReadRequest,
    ) -> open_arrow_access::Result<FileAccessor<Arc<dyn ByteAccessor>, F>> {
        let input = self
            .inputs
            .get(request.object.locator())
            .ok_or_else(crate::error::unknown_selected_file)?;
        request.object = input.path.clone().into();
        let bytes = crate::metrics::ObservedBytes::new(
            OpenDalAccessor::new(input.operator.clone()),
            self.metrics.clone(),
        );
        let bytes: Arc<dyn ByteAccessor> = match self.compression {
            OuterCompression::None => Arc::new(bytes),
            OuterCompression::Auto => Arc::new(bytes.layer(AutoDecompressionLayer::new())),
        };
        Ok(FileAccessor::from_format(bytes, self.format.clone()))
    }
}
#[async_trait]
impl<F> Accessor for Files<F>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + Clone + 'static,
    FileAccessor<Arc<dyn ByteAccessor>, F>: ScanPlanner<Request = ReadRequest>,
{
    type Request = ReadRequest;
    type Reader = dyn RecordReader;
    fn capability(&self) -> RecordCapability {
        self.format.capability(open_arrow_access::Capability {
            read: true,
            read_with_range: true,
            read_parallel: false,
            list: false,
        })
    }
    async fn metadata(
        &self,
        mut request: ReadRequest,
    ) -> open_arrow_access::Result<RecordMetadata> {
        self.file(&mut request)?.metadata(request).await
    }
    async fn open(
        &self,
        mut request: ReadRequest,
    ) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        self.file(&mut request)?.open(request).await
    }
}

#[async_trait]
impl<F> ScanPlanner for Files<F>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + Clone + 'static,
    FileAccessor<Arc<dyn ByteAccessor>, F>: ScanPlanner<Request = ReadRequest>,
{
    async fn plan_scan(&self, mut request: ReadRequest) -> open_arrow_access::Result<ScanPlan> {
        self.file(&mut request)?.plan_scan(request).await
    }
}
#[async_trait]
impl<F> StatisticsProvider for Files<F>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + Clone + 'static,
    FileAccessor<Arc<dyn ByteAccessor>, F>: ScanPlanner<Request = ReadRequest>,
{
    async fn statistics(&self, request: ReadRequest) -> open_arrow_access::Result<ScanStatistics> {
        let input = self
            .inputs
            .get(request.object.locator())
            .ok_or_else(crate::error::unknown_selected_file)?;
        let metadata = OpenDalAccessor::new(input.operator.clone())
            .metadata(open_arrow_access::ByteReadRequest::new(input.path.clone()))
            .await?;
        Ok(ScanStatistics {
            source_bytes: metadata
                .content_length
                .map_or(Statistic::Unknown, Statistic::Exact),
            ..Default::default()
        })
    }
}

pub(crate) async fn open<F>(
    locator: &str,
    config: &StorageConfig,
    options: ReadOptions,
    format: F,
    compression: OuterCompression,
) -> Result<DuckportSource>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + Clone + 'static,
    FileAccessor<Arc<dyn ByteAccessor>, F>: ScanPlanner<Request = ReadRequest>,
{
    if config.files.len() > 1 {
        config.metrics.set_indeterminate();
    }
    let mut objects = Vec::with_capacity(config.files.len());
    let mut inputs = HashMap::new();
    if config.files.is_empty() {
        let (operator, path) = crate::storage::operator(locator, config)?;
        objects.push(locator.to_owned());
        inputs.insert(locator.to_owned(), Input { operator, path });
    }
    for file in &config.files {
        let (operator, path) = crate::storage::operator(&file.locator, &file.storage)?;
        let operator = crate::storage::observe(operator, config);
        objects.push(file.locator.clone());
        inputs.insert(file.locator.clone(), Input { operator, path });
    }
    let files = FileSetAccessor::new(Files {
        inputs,
        format,
        compression,
        metrics: config.metrics.clone(),
    })
    .with_file_planning();
    let request = FileSetRequest::new(objects).with_options(options);
    // Check complete schemas before rows are emitted, including columns omitted
    // by projection or COUNT(*).
    if config.files.len() > 1 {
        files.metadata(request.clone()).await?;
    }
    DuckportSource::with_statistics(files, request).await
}
