//! Decode one explicitly selected provider attachment. File membership stays with the provider.
use crate::{error::Result, source::DuckportSource, validation};
use arrow_array::{ArrayRef, LargeBinaryArray};
use arrow_schema::{DataType, Field, Schema};
use open_arrow_access::{
    AccessorExt, ByteAccessor, Capability, DocumentAccessor, DocumentDecoder, OrderRequirement,
    ReadOptions, ReadRequest, RecordBatch, RecordCapability, SchemaRef, async_trait,
    decompression::AutoDecompressionLayer,
    file_format::{
        FileAccessor,
        csv::{CsvDialect, CsvFileFormat, CsvInferenceOptions},
        json::JsonFormat,
        parquet::ParquetAccessor,
        xml::XmlFormat,
    },
};
use std::sync::Arc;

pub async fn open<A: ByteAccessor + 'static>(
    files: A,
    file: String,
    format: &str,
    record_path: Option<String>,
    options: ReadOptions,
) -> Result<DuckportSource> {
    validation::validate_format(format, record_path.as_deref())?;
    let request = ReadRequest::new(file).with_options(options);
    if format == "binary" {
        return DuckportSource::new(DocumentAccessor::new(files, BinaryDecoder), request).await;
    }
    let files = files.layer(AutoDecompressionLayer::new());
    match format {
        "csv" | "tsv" => {
            let format = CsvFileFormat::infer()
                .with_dialect(CsvDialect {
                    delimiter: if format == "tsv" { b'\t' } else { b',' },
                    ..Default::default()
                })
                .with_inference_options(CsvInferenceOptions {
                    has_header: Some(true),
                    ..Default::default()
                });
            DuckportSource::new(FileAccessor::from_format(files, format), request).await
        }
        "json" | "jsonl" => {
            DuckportSource::new(
                FileAccessor::from_format(files, JsonFormat::infer()),
                request,
            )
            .await
        }
        "parquet" => DuckportSource::new(ParquetAccessor::new(files), request).await,
        "xml" => {
            DuckportSource::new(
                FileAccessor::from_format(files, XmlFormat::new(record_path.unwrap())),
                request,
            )
            .await
        }
        _ => unreachable!("validated format"),
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
