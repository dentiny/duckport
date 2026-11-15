//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, options::AttachmentFormat, source::DuckportSource, validation};
use open_arrow_access::{
    AttachmentFormat as SourceFormat, AttachmentOptions, ByteAccessor, OuterCompression,
    ReadOptions, ReadRequest, open_attachment_source,
};

/// Decode one selected attachment into Arrow records.
///
/// `record_path` selects the XML elements that become individual rows, for example
/// `/records/row`. It must be nonempty for XML and `None` for all other formats.
pub async fn open<A: ByteAccessor + 'static>(
    attachment_accessor: A,
    file: String,
    format: AttachmentFormat,
    record_path: Option<String>,
    options: ReadOptions,
) -> Result<DuckportSource> {
    validation::validate_format(format, record_path.as_deref())?;
    let format = match format {
        AttachmentFormat::Csv => SourceFormat::Csv,
        AttachmentFormat::Tsv => SourceFormat::Tsv,
        AttachmentFormat::Json => SourceFormat::Json,
        AttachmentFormat::Jsonl => SourceFormat::Jsonl,
        AttachmentFormat::Parquet => SourceFormat::Parquet,
        AttachmentFormat::Xml => SourceFormat::Xml {
            record_path: record_path.unwrap(),
        },
    };
    Ok(open_attachment_source(
        attachment_accessor,
        ReadRequest::new(file).with_options(options),
        AttachmentOptions::new(format).with_outer_compression(OuterCompression::Auto),
    )
    .await?
    .into())
}
