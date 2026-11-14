//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, options::AttachmentFormat, source::DuckportSource, validation};
use open_arrow_access::{
    AccessorExt, ByteAccessor, ReadOptions, ReadRequest,
    decompression::AutoDecompressionLayer,
    file_format::{
        FileAccessor,
        csv::{CsvDialect, CsvFileFormat, CsvHeader},
        json::JsonFormat,
        parquet::ParquetAccessor,
        xml::XmlFormat,
    },
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
    let request = ReadRequest::new(file).with_options(options);
    match format {
        AttachmentFormat::Csv | AttachmentFormat::Tsv => {
            let attachment_accessor = attachment_accessor.layer(AutoDecompressionLayer::new());
            let format = CsvFileFormat::infer().with_dialect(CsvDialect {
                header: CsvHeader::Present,
                delimiter: if format == AttachmentFormat::Tsv {
                    b'\t'
                } else {
                    b','
                },
                ..Default::default()
            });
            DuckportSource::new(
                FileAccessor::from_format(attachment_accessor, format),
                request,
            )
            .await
        }
        AttachmentFormat::Json | AttachmentFormat::Jsonl => {
            let attachment_accessor = attachment_accessor.layer(AutoDecompressionLayer::new());
            DuckportSource::new(
                FileAccessor::from_format(attachment_accessor, JsonFormat::infer()),
                request,
            )
            .await
        }
        AttachmentFormat::Parquet => {
            let attachment_accessor = attachment_accessor.layer(AutoDecompressionLayer::new());
            DuckportSource::new(ParquetAccessor::new(attachment_accessor), request).await
        }
        AttachmentFormat::Xml => {
            let attachment_accessor = attachment_accessor.layer(AutoDecompressionLayer::new());
            DuckportSource::new(
                FileAccessor::from_format(
                    attachment_accessor,
                    XmlFormat::new(record_path.unwrap()),
                ),
                request,
            )
            .await
        }
    }
}
