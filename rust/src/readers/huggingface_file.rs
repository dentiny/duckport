use crate::{
    ffi::{DuckportSource, open_reader},
    options::{ScanOptions, StorageOptions},
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::{
    ReadRequest,
    file_format::{
        FileAccessor,
        atif::AtifFormat,
        csv::{CsvDialect, CsvFileFormat, CsvInferenceOptions},
        json::JsonFormat,
        parquet::ParquetFormat,
        warc::WarcArrowFormat,
        xml::XmlFormat,
    },
    service::huggingface::*,
};
use std::ffi::c_char;

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_huggingface_file_open(
    locator: *const c_char,
    storage_options: *const StorageOptions,
    scan_options: *const ScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_reader(
            locator,
            storage_options,
            scan_options,
            output,
            |locator, storage_options, scan_options| async move {
                let mut settings = HuggingFaceOptions::default();
                if let Some(endpoint) = storage_options.endpoint {
                    settings = settings.with_endpoint(endpoint);
                }
                if let Some(token) = storage_options.token {
                    settings = settings.with_token(token);
                }
                let service = HuggingFaceService::new(settings).await?;
                let mut repository = HuggingFaceRepository::new(locator);
                if let Some(revision) = scan_options.revision {
                    repository = repository.with_revision(revision);
                }
                let path = scan_options.path.ok_or("path is required")?;
                let format = scan_options.format.ok_or("format is required")?;
                let storage = service.repository(repository).await?;
                let request = ReadRequest::new(path).with_options(scan_options.read_options);
                match format.as_str() {
                    "parquet" => {
                        AsyncSource::new(FileAccessor::from_format(storage, ParquetFormat), request)
                            .await
                    }
                    "csv" | "tsv" => {
                        let dialect = CsvDialect {
                            delimiter: if format == "csv" { b',' } else { b'\t' },
                            ..Default::default()
                        };
                        let format = CsvFileFormat::infer()
                            .with_dialect(dialect)
                            .with_inference_options(CsvInferenceOptions {
                                has_header: scan_options.header,
                                ..Default::default()
                            });
                        AsyncSource::new(FileAccessor::from_format(storage, format), request).await
                    }
                    "json" | "jsonl" | "ndjson" => {
                        AsyncSource::new(
                            FileAccessor::from_format(storage, JsonFormat::infer()),
                            request,
                        )
                        .await
                    }
                    "xml" => {
                        AsyncSource::new(
                            FileAccessor::from_format(
                                storage,
                                XmlFormat::new(
                                    scan_options.record_path.ok_or("record_path is required")?,
                                ),
                            ),
                            request,
                        )
                        .await
                    }
                    "atif" => {
                        AsyncSource::new(
                            FileAccessor::from_format(storage, AtifFormat::Json),
                            request,
                        )
                        .await
                    }
                    "atif_jsonl" => {
                        AsyncSource::new(
                            FileAccessor::from_format(storage, AtifFormat::JsonLines),
                            request,
                        )
                        .await
                    }
                    "warc" => {
                        AsyncSource::new(
                            FileAccessor::from_format(storage, WarcArrowFormat),
                            request,
                        )
                        .await
                    }
                    _ => Err(format!("unsupported format: {format}").into()),
                }
            },
        )
    }
}
