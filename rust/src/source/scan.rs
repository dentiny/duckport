//! Adapt a retained upstream plan to the upstream parallel-read executor.
use open_arrow_access::{
    Accessor, PreparedSource, ReadOptions, RecordCapability, RecordMetadata, RecordReader, Result,
    ScanPlan, ScanPlanner, async_trait,
};
use std::sync::Arc;

pub(super) struct PreparedScan(pub Arc<PreparedSource>, pub RecordCapability);
#[async_trait]
impl Accessor for PreparedScan {
    type Request = ReadOptions;
    type Reader = dyn RecordReader;
    fn capability(&self) -> RecordCapability {
        self.1
    }
    async fn metadata(&self, options: ReadOptions) -> Result<RecordMetadata> {
        let mut metadata = self.0.metadata().clone();
        metadata.schema = options.resolve_projection(&self.0.schema())?.schema;
        Ok(metadata)
    }
    async fn open(&self, options: ReadOptions) -> Result<Box<dyn RecordReader>> {
        self.0.scan(options).await
    }
}
#[async_trait]
impl ScanPlanner for PreparedScan {
    async fn plan_scan(&self, options: ReadOptions) -> Result<ScanPlan> {
        self.0.plan_scan(options).await
    }
}
