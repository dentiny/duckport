//! Adapt a retained upstream plan to integrated crawl execution.
use open_arrow_access::{
    Accessor, PreparedSource, ReadOptions, ReadOptionsProvider, RecordCapability, RecordMetadata,
    RecordReader, Result, ScanPlan, ScanPlanner, async_trait,
};
use std::sync::Arc;

pub(super) struct PreparedScan(pub Arc<PreparedSource>, pub RecordCapability);

// Schema evolution may widen the projection before opening the retained plan.
#[derive(Clone)]
pub(super) struct ScanRequest(pub ReadOptions);

impl ReadOptionsProvider for ScanRequest {
    fn read_options(&self) -> &ReadOptions {
        &self.0
    }

    fn read_options_mut(&mut self) -> &mut ReadOptions {
        &mut self.0
    }
}

#[async_trait]
impl Accessor for PreparedScan {
    type Request = ScanRequest;
    type Reader = dyn RecordReader;
    fn capability(&self) -> RecordCapability {
        self.1
    }
    async fn metadata(&self, request: ScanRequest) -> Result<RecordMetadata> {
        let mut metadata = self.0.metadata().clone();
        metadata.schema = request.0.resolve_projection(&self.0.schema())?.schema;
        Ok(metadata)
    }
    async fn open(&self, request: ScanRequest) -> Result<Box<dyn RecordReader>> {
        self.0.scan(request.0).await
    }
}
#[async_trait]
impl ScanPlanner for PreparedScan {
    async fn plan_scan(&self, request: ScanRequest) -> Result<ScanPlan> {
        self.0.plan_scan(request.0).await
    }
}
