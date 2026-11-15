//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, source::DuckportSource};
use open_arrow_access::{
    AttachmentFormat, AttachmentOptions, ByteAccessor, OuterCompression, ReadOptions, ReadRequest,
    open_attachment_source,
};

pub fn attachment_options(format: &str, record_path: Option<&str>) -> Result<AttachmentOptions> {
    let selection = match format {
        "binary" => AttachmentFormat::Binary,
        "csv" => AttachmentFormat::Csv,
        "tsv" => AttachmentFormat::Tsv,
        "json" => AttachmentFormat::Json,
        "jsonl" => AttachmentFormat::Jsonl,
        "parquet" => AttachmentFormat::Parquet,
        "xml" => AttachmentFormat::Xml {
            record_path: record_path
                .filter(|path| !path.is_empty())
                .ok_or("record_path is required for XML attachments")?
                .to_owned(),
        },
        _ => return Err("format must be binary, csv, tsv, json, jsonl, parquet, or xml".into()),
    };
    if format != "xml" && record_path.is_some() {
        return Err("record_path is only supported for XML attachments".into());
    }
    let compression = if format == "binary" {
        OuterCompression::None
    } else {
        OuterCompression::Auto
    };
    let options = AttachmentOptions::new(selection).with_outer_compression(compression);
    options.validate()?;
    Ok(options)
}

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
