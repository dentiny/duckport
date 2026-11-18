//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, source::DuckportSource};
use open_arrow_access::{
    Accessor, AttachmentOptions, ByteAccessor, OrderRequirement, ReadOptions, ReadRequest,
    RecordCapability, RecordReader, async_trait, open_attachment_source,
};
use std::sync::Arc;

// Keep the provider-owned byte accessor so each scan can pass its projection to
// the existing attachment decoder without resolving file membership again.
struct AttachmentAccessor<A> {
    accessor: Arc<A>,
    options: AttachmentOptions,
}

#[async_trait]
impl<A: ByteAccessor + 'static> Accessor for AttachmentAccessor<A> {
    type Request = ReadRequest;
    type Reader = dyn RecordReader;

    fn capability(&self) -> RecordCapability {
        RecordCapability::sequential(OrderRequirement::PreserveGlobal)
    }

    async fn open(&self, request: ReadRequest) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        open_attachment_source(self.accessor.clone(), request, self.options.clone())
            .await?
            .reader()
            .await
    }
}

/// Decode one selected attachment into Arrow records, preserving binary bytes unchanged.
pub async fn open<A: ByteAccessor + 'static>(
    attachment_accessor: A,
    file: String,
    attachment_options: AttachmentOptions,
    options: ReadOptions,
    metrics: crate::metrics::ReadMetrics,
) -> Result<DuckportSource> {
    DuckportSource::new(
        AttachmentAccessor {
            accessor: Arc::new(crate::metrics::ObservedBytes::new(
                attachment_accessor,
                metrics,
            )),
            options: attachment_options,
        },
        ReadRequest::new(file).with_options(options),
    )
    .await
}
