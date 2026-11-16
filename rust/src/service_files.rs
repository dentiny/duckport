//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, source::DuckportSource};
use open_arrow_access::{
    AttachmentOptions, ByteAccessor, ReadOptions, ReadRequest, open_attachment_source,
};

/// Decode one selected attachment into Arrow records, preserving binary bytes unchanged.
pub async fn open<A: ByteAccessor + 'static>(
    attachment_accessor: A,
    file: String,
    attachment_options: AttachmentOptions,
    options: ReadOptions,
) -> Result<DuckportSource> {
    Ok(open_attachment_source(
        attachment_accessor,
        ReadRequest::new(file).with_options(options),
        attachment_options,
    )
    .await?
    .into())
}
