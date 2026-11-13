use open_arrow_access::{
    Accessor, ByteReadRequest, ByteReader, ReadOptions, ReadRequest, ReaderFactoryExt,
    RecordReader,
    decompression::{
        Compression, DecompressionFactory, bz2::Bzip2, gzip::Gzip, lz4::Lz4, zstd::Zstd,
    },
    file_format::{
        atif::AtifFormat,
        csv::{CsvDialect, CsvFileFormat, CsvInferenceOptions},
        json::JsonFormat,
        parquet::ParquetFormat,
        warc::WarcArrowFormat,
        xml::XmlFormat,
    },
    service::huggingface::{
        HuggingFaceDatasetRequest, HuggingFaceOptions, HuggingFaceRepository, HuggingFaceService,
    },
    storage::{OpenDalAccessor, opendal},
};
use serde::Deserialize;
use std::{collections::HashMap, error::Error, num::NonZeroUsize, path::Path};
use url::Url;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    storage_options: HashMap<String, String>,
    format: Option<String>,
    header: Option<bool>,
    record_path: Option<String>,
    config: Option<String>,
    split: Option<String>,
    revision: Option<String>,
    endpoint: Option<String>,
    token: Option<String>,
    path: Option<String>,
    allow_partial: bool,
}

/// Each SQL reader calls a typed upstream API; only the Arrow bridge is shared.
pub async fn open(
    reader: &str,
    locator: &str,
    options: Options,
    batch_size: usize,
) -> Result<Box<dyn RecordReader>> {
    if locator.is_empty() {
        return Err("source must not be empty".into());
    }
    let read_options = ReadOptions {
        batch_size: NonZeroUsize::new(batch_size).ok_or("batch_size must be positive")?,
        ..Default::default()
    };
    if reader == "huggingface" || reader == "huggingface_file" {
        let mut settings = HuggingFaceOptions::default();
        if let Some(endpoint) = &options.endpoint {
            settings = settings.with_endpoint(endpoint);
        }
        if let Some(token) = &options.token {
            settings = settings.with_token(token);
        }
        let service = HuggingFaceService::new(settings).await?;
        if reader == "huggingface" {
            let mut request = HuggingFaceDatasetRequest::new(
                locator,
                options.config.as_deref().unwrap_or("default"),
                options.split.as_deref().unwrap_or("train"),
            )
            .with_options(read_options);
            if let Some(revision) = options.revision {
                request = request.with_revision(revision);
            }
            if options.allow_partial {
                request = request.with_partial_exports();
            }
            return Ok(service.dataset(request).await?);
        }
        let mut repository = HuggingFaceRepository::new(locator);
        if let Some(revision) = &options.revision {
            repository = repository.with_revision(revision);
        }
        let path = options.path.as_deref().ok_or("path is required")?;
        let files = service.repository(repository).await?;
        let input = files.open(ByteReadRequest::new(path)).await?;
        return decode(
            options.format.as_deref().ok_or("format is required")?,
            input,
            path,
            &options,
            read_options,
        )
        .await;
    }
    let format = match reader {
        "fs" | "http" | "s3" => {
            let matches = match reader {
                "fs" => !locator.contains("://") || locator.starts_with("file://"),
                "http" => locator.starts_with("http://") || locator.starts_with("https://"),
                _ => locator.starts_with("s3://"),
            };
            if !matches {
                return Err(
                    format!("read_{reader}: source uses a different storage backend").into(),
                );
            }
            options.format.as_deref().ok_or("format is required")?
        }
        format => format,
    };
    let (operator, path) = storage(locator, options.storage_options.clone())?;
    let input = OpenDalAccessor::new(operator)
        .open(ByteReadRequest::new(path.clone()))
        .await?;
    decode(format, input, &path, &options, read_options).await
}

async fn decode(
    format: &str,
    input: Box<dyn ByteReader>,
    path: &str,
    options: &Options,
    read_options: ReadOptions,
) -> Result<Box<dyn RecordReader>> {
    let input = match Compression::from_path(path)? {
        Compression::Uncompressed => input,
        Compression::Gzip => {
            DecompressionFactory::new(Gzip)
                .open_input(input, path.into())
                .await?
        }
        Compression::Bzip2 => {
            DecompressionFactory::new(Bzip2)
                .open_input(input, path.into())
                .await?
        }
        Compression::Lz4 => {
            DecompressionFactory::new(Lz4)
                .open_input(input, path.into())
                .await?
        }
        Compression::Zstd => {
            DecompressionFactory::new(Zstd)
                .open_input(input, path.into())
                .await?
        }
    };
    let request = ReadRequest::new(path).with_options(read_options);
    let reader = match format {
        "parquet" => ParquetFormat.open_input(input, request).await?,
        "csv" | "tsv" => {
            CsvFileFormat::infer()
                .with_dialect(CsvDialect {
                    delimiter: if format == "tsv" { b'\t' } else { b',' },
                    ..Default::default()
                })
                .with_inference_options(CsvInferenceOptions {
                    has_header: options.header,
                    ..Default::default()
                })
                .open_input(input, request)
                .await?
        }
        "json" | "jsonl" | "ndjson" => JsonFormat::infer().open_input(input, request).await?,
        "xml" => {
            XmlFormat::new(
                options
                    .record_path
                    .as_deref()
                    .ok_or("record_path is required")?,
            )
            .open_input(input, request)
            .await?
        }
        "atif" => AtifFormat::Json.open_input(input, request).await?,
        "atif_jsonl" => AtifFormat::JsonLines.open_input(input, request).await?,
        "warc" => WarcArrowFormat.open_input(input, request).await?,
        _ => return Err(format!("unsupported format: {format}").into()),
    };
    Ok(reader)
}

fn storage(
    locator: &str,
    mut config: HashMap<String, String>,
) -> Result<(opendal::Operator, String)> {
    opendal::install_default();
    if !locator.contains("://") || locator.starts_with("file://") {
        if config.contains_key("root") {
            return Err("file path already determines storage root".into());
        }
        let path = if locator.starts_with("file://") {
            Url::parse(locator)?
                .to_file_path()
                .map_err(|_| "invalid local file URI")?
        } else {
            Path::new(locator).to_path_buf()
        };
        let path = std::path::absolute(path)?;
        config.insert(
            "root".into(),
            path.parent()
                .ok_or("source must name a file")?
                .to_str()
                .ok_or("path must be UTF-8")?
                .into(),
        );
        return Ok((
            opendal::Operator::via_iter("fs", config)?,
            path.file_name()
                .and_then(|s| s.to_str())
                .ok_or("source must name a UTF-8 file")?
                .into(),
        ));
    }
    let mut url = Url::parse(locator)?;
    if url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("URI queries, fragments, and embedded credentials are unsupported".into());
    }
    let scheme = url.scheme().to_owned();
    if !matches!(scheme.as_str(), "http" | "https" | "s3") {
        return Err(format!("unsupported storage scheme: {scheme}").into());
    }
    let path = percent_encoding::percent_decode_str(url.path().trim_start_matches('/'))
        .decode_utf8()?
        .into_owned();
    if config.contains_key("root") {
        return Err("URI already determines storage root".into());
    }
    url.set_path("/");
    let operator = if scheme == "http" || scheme == "https" {
        if config.contains_key("endpoint") {
            return Err("HTTP URI already determines endpoint".into());
        }
        config.insert("endpoint".into(), url.as_str().trim_end_matches('/').into());
        opendal::Operator::via_iter("http", config)?
    } else {
        opendal::Operator::from_uri(opendal::OperatorUri::new(url.as_str(), config)?)?
    };
    Ok((operator, path))
}
