//! Parsing, normalization and object construction; input checks live in validation.
use crate::{error::Result, options::StorageConfig, validation};
use arrow_schema::{DataType, Schema};
use open_arrow_access::service::arxiv::ArxivKind;
use open_arrow_access::service::biorxiv::Server;
use open_arrow_access::service::huggingface::HuggingFaceDatasetRequest;
use open_arrow_access::service::regulations::RegulationsResource;
use open_arrow_access::{AttachmentFormat, AttachmentOptions, OuterCompression};
use open_arrow_sink_core::{CompressionSpec, IndexSpec, IndexType, StructuralEncoding, WriteMode};
use std::num::NonZeroUsize;
use url::Url;

/// # Safety
/// A non-null pointer must reference a live NUL-terminated string.
pub(crate) unsafe fn string(value: *const std::ffi::c_char) -> Result<String> {
    validation::non_null(value, "null string argument")?;
    Ok(unsafe { std::ffi::CStr::from_ptr(value) }
        .to_str()?
        .to_owned())
}

/// # Safety
/// A non-null pointer must reference a live NUL-terminated string.
pub(crate) unsafe fn optional_string(value: *const std::ffi::c_char) -> Result<Option<String>> {
    if value.is_null() {
        Ok(None)
    } else {
        unsafe { string(value) }.map(Some)
    }
}

/// # Safety
/// For nonzero length, the pointer must reference len readable, aligned values.
pub(crate) unsafe fn ffi_slice<'a, T>(
    value: *const T,
    len: usize,
    message: &str,
) -> Result<&'a [T]> {
    if len == 0 {
        return Ok(&[]);
    }
    validation::non_null(value, message)?;
    Ok(unsafe { std::slice::from_raw_parts(value, len) })
}

pub(crate) fn local_path(locator: &str) -> Result<std::path::PathBuf> {
    let path = if locator.starts_with("file://") {
        Url::parse(locator)?
            .to_file_path()
            .map_err(|_| "invalid local file URI")?
    } else {
        std::path::Path::new(locator).to_path_buf()
    };
    Ok(std::path::absolute(path)?)
}

pub(crate) fn commoncrawl_location(locator: &str) -> Result<(std::path::PathBuf, String)> {
    let path = local_path(locator)?;
    let root = validation::required(
        path.ancestors()
            .find(|parent| parent.file_name().is_some_and(|name| name == "crawl-data"))
            .and_then(|directory| directory.parent()),
        "local Common Crawl path must contain crawl-data/<crawl>",
    )?;
    let key = path
        .strip_prefix(root)?
        .iter()
        .map(|part| validation::required(part.to_str(), "path must be UTF-8"))
        .collect::<Result<Vec<_>>>()?
        .join("/");
    Ok((root.to_path_buf(), key))
}

pub(crate) fn batch_size(value: usize) -> Result<NonZeroUsize> {
    validation::validate_batch_size(value)?;
    validation::required(NonZeroUsize::new(value), validation::BATCH_SIZE_ERROR)
}

pub(crate) fn boolean(name: &str, value: i32) -> Result<bool> {
    validation::validate_boolean(name, value)?;
    Ok(value == 1)
}

pub(crate) fn sink_mode(value: &str) -> Result<open_arrow_sink_core::WriteMode> {
    validation::required(
        match value {
            "create" => Some(WriteMode::Create),
            "append" => Some(WriteMode::Append),
            "overwrite" => Some(WriteMode::Overwrite),
            _ => None,
        },
        "write mode must be create, append, or overwrite",
    )
}

pub(crate) fn arxiv_kind(value: &str) -> Result<open_arrow_access::service::arxiv::ArxivKind> {
    validation::required(
        match value {
            "source" => Some(ArxivKind::Source),
            "pdf" => Some(ArxivKind::Pdf),
            _ => None,
        },
        "kind must be source or pdf",
    )
}

pub(crate) fn biorxiv_server(value: &str) -> Result<open_arrow_access::service::biorxiv::Server> {
    validation::required(
        match value {
            "biorxiv" => Some(Server::Biorxiv),
            "medrxiv" => Some(Server::Medrxiv),
            _ => None,
        },
        "server must be biorxiv or medrxiv",
    )
}

pub(crate) fn regulations_resource(
    value: &str,
) -> Result<open_arrow_access::service::regulations::RegulationsResource> {
    validation::required(
        match value {
            "documents" => Some(RegulationsResource::Documents),
            "comments" => Some(RegulationsResource::Comments),
            "dockets" => Some(RegulationsResource::Dockets),
            _ => None,
        },
        "resource must be documents, comments, or dockets",
    )
}
pub(crate) fn sink_url(path: &str) -> Result<Url> {
    let mut url = if path.contains("://") {
        Url::parse(path)?
    } else {
        Url::from_directory_path(local_path(path)?).map_err(|_| "invalid dataset path")?
    };
    validation::validate_uri(&url)?;
    validation::validate_sink_scheme(&url)?;
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

pub(crate) fn sanitize_storage_root(mut root: Url) -> Url {
    if root.scheme() == "s3" {
        root.set_path("/");
    }
    root
}

pub(crate) fn internet_archive_credentials(config: &StorageConfig) -> Result<Option<(&str, &str)>> {
    validation::validate_internet_archive_credentials(config)?;
    Ok(config
        .security
        .access_key_id
        .as_deref()
        .zip(config.security.secret_access_key.as_deref()))
}

pub(crate) fn courtlistener_header(
    header: &csv_async::StringRecord,
) -> Result<arrow_schema::Schema> {
    validation::validate_courtlistener_header(header)?;
    Ok(arrow_schema::Schema::new(
        header
            .iter()
            .map(|name| arrow_schema::Field::new(name, arrow_schema::DataType::Utf8, true))
            .collect::<Vec<_>>(),
    ))
}
// The service takes a typed request; adapt a fully qualified split URI at the SQL boundary.
pub(crate) fn huggingface_request(
    locator: &str,
) -> Result<open_arrow_access::service::huggingface::HuggingFaceDatasetRequest> {
    let url = url::Url::parse(locator)?;
    validation::validate_huggingface_uri(&url)?;
    let path = validation::required(
        locator.strip_prefix("hf://datasets/"),
        "expected a fully qualified hf://datasets/ URL",
    )?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let parts = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
    let [owner, repository, config, split]: [&str; 4] = validation::required(
        parts.try_into().ok(),
        "Hugging Face URL must select a config/split directory",
    )?;
    let (repository, revision) = validation::required(
        repository.split_once('@'),
        "Hugging Face URL must include @<revision>",
    )?;
    let decode = |value: &str| -> Result<String> {
        Ok(percent_encoding::percent_decode_str(value)
            .decode_utf8()?
            .into_owned())
    };
    let owner = decode(owner)?;
    let repository = decode(repository)?;
    let revision = decode(revision)?;
    let config = decode(config)?;
    let split = decode(split)?;
    crate::validation::validate_huggingface_path(&owner, &repository, &revision, &config, &split)?;
    Ok(
        HuggingFaceDatasetRequest::new(format!("{owner}/{repository}"), config, split)
            .with_revision(if revision == "~parquet" {
                "refs/convert/parquet".into()
            } else {
                revision
            }),
    )
}

pub(crate) fn attachment_options(
    format: &str,
    record_path: Option<&str>,
) -> Result<open_arrow_access::AttachmentOptions> {
    let selection = match format {
        "binary" => Some(AttachmentFormat::Binary),
        "csv" => Some(AttachmentFormat::Csv),
        "tsv" => Some(AttachmentFormat::Tsv),
        "json" => Some(AttachmentFormat::Json),
        "jsonl" => Some(AttachmentFormat::Jsonl),
        "parquet" => Some(AttachmentFormat::Parquet),
        "xml" => Some(AttachmentFormat::Xml {
            record_path: validation::nonempty(
                record_path,
                "record_path is required for XML attachments",
            )?
            .to_owned(),
        }),
        _ => None,
    };
    let selection = validation::required(
        selection,
        "format must be binary, csv, tsv, json, jsonl, parquet, or xml",
    )?;
    validation::validate_attachment_record_path(format, record_path)?;
    let compression = if format == "binary" {
        OuterCompression::None
    } else {
        OuterCompression::Auto
    };
    let options = AttachmentOptions::new(selection).with_outer_compression(compression);
    validation::validate_attachment_options(&options)?;
    Ok(options)
}

pub(crate) fn byte_size(name: &str, value: i64) -> Result<usize> {
    validation::validate_byte_size(name, value)?;
    Ok(usize::try_from(value).map_err(|_| format!("{name} exceeds platform size"))?)
}

pub(crate) fn optional_byte_size(name: &str, value: i64) -> Result<Option<usize>> {
    if value == -1 {
        return Ok(None);
    }
    byte_size(name, value).map(Some)
}

pub(crate) fn resolve_column_path(schema: &Schema, name: &str, option: &str) -> Result<Vec<String>> {
    if let Some(field) = schema
        .fields()
        .iter()
        .find(|field| field.name().eq_ignore_ascii_case(name))
    {
        return Ok(vec![field.name().clone()]);
    }
    let mut fields = schema.fields();
    let mut path = Vec::new();
    let mut parts = name.split('.').peekable();
    while let Some(part) = parts.next() {
        let field = fields
            .iter()
            .find(|field| field.name().eq_ignore_ascii_case(part))
            .ok_or_else(|| format!("{option} field not found: {name}"))?;
        path.push(field.name().clone());
        if parts.peek().is_none() {
            break;
        }
        let mut data_type = field.data_type();
        loop {
            match data_type {
                DataType::List(item)
                | DataType::LargeList(item)
                | DataType::FixedSizeList(item, _) => {
                    data_type = item.data_type();
                }
                DataType::Struct(children) => {
                    fields = children;
                    break;
                }
                _ => return Err(format!("{option} field not found: {name}").into()),
            }
        }
    }
    Ok(path)
}

pub(crate) fn column_compression(
    column: &str,
    codec: Option<&str>,
    level: Option<i32>,
) -> Result<Option<CompressionSpec>> {
    let compression = match codec {
        None => None,
        Some(codec) => Some(match codec.to_ascii_lowercase().as_str() {
            "none" | "uncompressed" => CompressionSpec::Uncompressed,
            "zstd" => CompressionSpec::Zstd { level },
            "lz4" | "lz4_raw" => CompressionSpec::Lz4Raw,
            "snappy" => CompressionSpec::Snappy,
            "gzip" => CompressionSpec::Gzip,
            _ => return Err(format!("Unknown COLUMN_COMPRESSION codec {codec}: {column}").into()),
        }),
    };
    validation::validate_column_compression_level(column, compression, level)?;
    Ok(compression)
}

pub(crate) fn column_structural_encoding(
    column: &str,
    path: &[String],
    encoding: &str,
) -> Result<StructuralEncoding> {
    validation::validate_structural_encoding_path(path)?;
    match encoding.to_ascii_lowercase().as_str() {
        "miniblock" => Ok(StructuralEncoding::Miniblock),
        "fullzip" => Ok(StructuralEncoding::Fullzip),
        _ => {
            Err(format!("COLUMN_STRUCTURAL_ENCODING must be miniblock or fullzip: {column}").into())
        }
    }
}

pub(crate) fn index_spec(schema: &Schema, column: &str, option: &str) -> Result<IndexSpec> {
    let field = schema
        .fields()
        .iter()
        .find(|field| field.name().eq_ignore_ascii_case(column))
        .ok_or_else(|| format!("{option} column not found: {column}"))?;
    let index_type = match option.to_ascii_lowercase().as_str() {
        "scalar_index_columns" => IndexType::Scalar,
        "vector_index_columns" => IndexType::Vector,
        "text_index_columns" => IndexType::Text,
        "bloom_filter_index_columns" => IndexType::BloomFilter,
        _ => return Err(format!("Unknown index option: {option}").into()),
    };
    Ok(IndexSpec::new(field.name().clone(), index_type))
}
