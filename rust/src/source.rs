use open_arrow_access::{
    Accessor, ByteReadRequest, ByteReader, ReadOptions, ReadRequest, ReaderFactoryExt,
    RecordReader,
    decompression::{
        Compression, DecompressionFactory, bz2::Bzip2, gzip::Gzip, lz4::Lz4, zstd::Zstd,
    },
    storage::{OpenDalAccessor, opendal},
};
use serde::Deserialize;
use std::{collections::HashMap, error::Error, num::NonZeroUsize, path::Path};
use url::Url;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

// Different reader modules consume different fields of this shared FFI payload.
#[allow(dead_code)]
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
    open_reader(reader, locator, options, read_options).await
}

async fn open_file(locator: &str, options: &Options) -> Result<(Box<dyn ByteReader>, String)> {
    let (operator, path) = storage(locator, options.storage_options.clone())?;
    let input = OpenDalAccessor::new(operator)
        .open(ByteReadRequest::new(path.clone()))
        .await?;
    let input = match Compression::from_path(&path)? {
        Compression::Uncompressed => input,
        Compression::Gzip => {
            DecompressionFactory::new(Gzip)
                .open_input(input, path.clone().into())
                .await?
        }
        Compression::Bzip2 => {
            DecompressionFactory::new(Bzip2)
                .open_input(input, path.clone().into())
                .await?
        }
        Compression::Lz4 => {
            DecompressionFactory::new(Lz4)
                .open_input(input, path.clone().into())
                .await?
        }
        Compression::Zstd => {
            DecompressionFactory::new(Zstd)
                .open_input(input, path.clone().into())
                .await?
        }
    };
    Ok((input, path))
}

// Generated from the reader modules present in this checkout; no shared registry edits.
include!(concat!(env!("OUT_DIR"), "/readers.rs"));

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
