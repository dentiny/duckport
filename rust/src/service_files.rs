//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, source::DuckportSource};
use open_arrow_access::{
    AttachmentAccessor, AttachmentOptions, ByteAccessor, ReadOptions, ReadRequest,
};

/// Decode one selected attachment into Arrow records, preserving binary bytes unchanged.
pub async fn open<A: ByteAccessor + 'static>(
    attachment_accessor: A,
    file: String,
    attachment_options: AttachmentOptions,
    options: ReadOptions,
    metrics: crate::metrics::ReadMetrics,
) -> Result<DuckportSource> {
    DuckportSource::new(
        AttachmentAccessor::new(
            crate::metrics::ObservedBytes::new(attachment_accessor, metrics),
            attachment_options,
        )?,
        ReadRequest::new(file).with_options(options),
    )
    .await
}
