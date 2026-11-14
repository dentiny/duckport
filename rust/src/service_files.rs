//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, options::AttachmentFormat, source::DuckportSource, validation};
use arrow_array::{ArrayRef, LargeBinaryArray};
use arrow_schema::{DataType, Field, Schema};
use open_arrow_access::{
    AccessorExt, ByteAccessor, Capability, DocumentAccessor, DocumentDecoder, OrderRequirement,
    ReadOptions, ReadRequest, RecordBatch, RecordCapability, SchemaRef, async_trait,
    decompression::AutoDecompressionLayer,
    file_format::{
        FileAccessor,
        csv::{CsvDialect, CsvFileFormat, CsvHeader},
        json::JsonFormat,
        parquet::ParquetAccessor,
        xml::XmlFormat,
    },
};
use std::sync::Arc;

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
        AttachmentFormat::Binary => {
            DuckportSource::new(
                DocumentAccessor::new(attachment_accessor, BinaryDecoder),
                request,
            )
            .await
        }
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

struct BinaryDecoder;
#[async_trait]
impl DocumentDecoder for BinaryDecoder {
    type Request = ReadRequest;
    fn capability(&self, input: Capability) -> RecordCapability {
        Capability {
            read: input.read,
            read_with_range: false,
            read_parallel: false,
            list: false,
        }
        .with_ordering(OrderRequirement::Reorderable)
    }
    fn prepare(&self, _: &ReadRequest) -> open_arrow_access::Result<SchemaRef> {
        Ok(Arc::new(Schema::new(vec![Field::new(
            "data",
            DataType::LargeBinary,
            false,
        )])))
    }
    async fn decode(
        &self,
        raw: &[u8],
        _: ReadRequest,
        schema: SchemaRef,
    ) -> open_arrow_access::Result<RecordBatch> {
        let data = Arc::new(LargeBinaryArray::from(vec![raw])) as ArrayRef;
        RecordBatch::try_new(schema, vec![data]).map_err(|error| {
            open_arrow_access::Error::new(
                open_arrow_access::ErrorKind::Decode,
                "decode_attachment",
                error.to_string(),
            )
            .with_source(error)
        })
    }
}
