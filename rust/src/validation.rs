use crate::{error::Result, options::StorageConfig};
use url::Url;

pub(crate) fn validate_uri(url: &Url) -> Result<()> {
    if url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("URI queries, fragments, and embedded credentials are unsupported".into());
    }
    Ok(())
}

pub(crate) fn validate_storage_scheme(scheme: &str) -> Result<()> {
    if !matches!(scheme, "http" | "https" | "s3") {
        return Err(format!("unsupported storage scheme: {scheme}").into());
    }
    Ok(())
}

pub(crate) fn validate_http_config(config: &StorageConfig) -> Result<()> {
    if config.endpoint.is_some() {
        return Err("HTTP URI already determines endpoint".into());
    }
    Ok(())
}

pub(crate) fn validate_huggingface_path(
    owner: &str,
    repository: &str,
    revision: &str,
    config: &str,
    split: &str,
) -> Result<()> {
    if [owner, repository, config, split].iter().any(|value| {
        value.is_empty() || matches!(*value, "." | "..") || value.contains(['/', '\\', '\0'])
    }) || revision.is_empty()
    {
        return Err("Hugging Face URL contains an invalid path component".into());
    }
    Ok(())
}

pub(crate) fn validate_huggingface_url_options(
    revision: Option<&str>,
    config: Option<&str>,
    split: Option<&str>,
) -> Result<()> {
    if revision.is_some() || config.is_some() || split.is_some() {
        return Err("revision, config, and split cannot be combined with a fully qualified Hugging Face URL".into());
    }
    Ok(())
}

pub(crate) fn required<T>(value: Option<T>, message: &str) -> Result<T> {
    value.ok_or_else(|| message.into())
}

pub(crate) fn non_null<T>(value: *const T, message: &str) -> Result<()> {
    if value.is_null() {
        return Err(message.into());
    }
    Ok(())
}

pub(crate) fn validate_requester_pays(
    locator: &str,
    config: &StorageConfig,
    enabled: bool,
) -> Result<()> {
    if enabled && !locator.starts_with("s3://") {
        return Err("requester_pays requires an S3 root".into());
    }
    if enabled && (config.access_key_id.is_none() || config.secret_access_key.is_none()) {
        return Err("requester_pays requires a scoped S3 secret with credentials".into());
    }
    Ok(())
}

pub(crate) fn validate_oai_selectors(
    locator: &str,
    from: Option<&str>,
    until: Option<&str>,
    set: Option<&str>,
) -> Result<()> {
    if locator != "records" && (from.is_some() || until.is_some() || set.is_some()) {
        return Err("resumption tokens cannot be combined with date/set selectors".into());
    }
    Ok(())
}

pub(crate) fn validate_hathifiles_locator(locator: &str) -> Result<()> {
    if locator != "catalog" {
        return Err("Hathifiles locator must be catalog".into());
    }
    Ok(())
}

pub(crate) fn nonempty<T: AsRef<str>>(value: Option<T>, message: &str) -> Result<T> {
    required(value.filter(|value| !value.as_ref().is_empty()), message)
}

pub(crate) const BATCH_SIZE_ERROR: &str = "batch_size must be between 1 and 1048576";

pub(crate) fn validate_batch_size(value: usize) -> Result<()> {
    if !(1..=1048576).contains(&value) {
        return Err(BATCH_SIZE_ERROR.into());
    }
    Ok(())
}

pub(crate) fn validate_boolean(name: &str, value: i32) -> Result<()> {
    if !matches!(value, 0 | 1) {
        return Err(format!("{name} must be 0 or 1").into());
    }
    Ok(())
}

pub(crate) fn validate_sink_scheme(url: &Url) -> Result<()> {
    if !matches!(url.scheme(), "file" | "s3") {
        return Err("sink destination must be a local path, file:// URI, or s3:// URI".into());
    }
    Ok(())
}

pub(crate) fn validate_internet_archive_credentials(config: &StorageConfig) -> Result<()> {
    if config.access_key_id.is_some() != config.secret_access_key.is_some() {
        return Err("Internet Archive requires both an access key and a secret key".into());
    }
    Ok(())
}

pub(crate) fn validate_courtlistener_header(header: &csv_async::StringRecord) -> Result<()> {
    let mut names = std::collections::HashSet::new();
    for name in header {
        if name.is_empty() || !names.insert(name) {
            return Err("CourtListener CSV header names must be nonempty and unique".into());
        }
    }
    Ok(())
}

pub(crate) fn validate_huggingface_uri(url: &Url) -> Result<()> {
    validate_uri(url)?;
    if url.scheme() != "hf" || url.host_str() != Some("datasets") || url.port().is_some() {
        return Err("expected hf://datasets/<owner>/<repo>@<revision>/<config>/<split>/".into());
    }
    Ok(())
}

pub(crate) fn validate_attachment_record_path(
    format: &str,
    record_path: Option<&str>,
) -> Result<()> {
    if format != "xml" && record_path.is_some() {
        return Err("record_path is only supported for XML attachments".into());
    }
    Ok(())
}

pub(crate) fn validate_attachment_options(
    options: &open_arrow_access::AttachmentOptions,
) -> Result<()> {
    Ok(options.validate()?)
}

pub(crate) fn validate_iceberg_catalog_uri(uri: &Url) -> Result<()> {
    validate_uri(uri)?;
    if !matches!(uri.scheme(), "http" | "https") {
        return Err("CATALOG_URI must use http:// or https://".into());
    }
    Ok(())
}

pub(crate) fn validate_iceberg_identifier(identifier: &[&str]) -> Result<()> {
    if identifier.len() < 2 || identifier.iter().any(|part| part.is_empty()) {
        return Err("Iceberg destination must be namespace.table".into());
    }
    Ok(())
}

pub(crate) fn validate_iceberg_storage_root(root: &Url) -> Result<()> {
    if root.scheme() == "s3" && root.path() != "/" {
        return Err("Iceberg STORAGE_ROOT must be an S3 bucket root (s3://bucket/)".into());
    }
    Ok(())
}

pub(crate) fn validate_iceberg_table(
    table: &open_arrow_sink_iceberg::iceberg::table::Table,
) -> Result<()> {
    if !table
        .metadata()
        .default_partition_spec()
        .fields()
        .is_empty()
    {
        return Err("Iceberg COPY currently supports unpartitioned tables only".into());
    }
    Ok(())
}

pub(crate) fn validate_iceberg_schema(
    schema: &arrow_schema::Schema,
    output_schema: &arrow_schema::Schema,
) -> Result<()> {
    if schema.fields().len() != output_schema.fields().len()
        || schema
            .fields()
            .iter()
            .zip(output_schema.fields())
            .any(|(input, output)| {
                input.name() != output.name()
                    || !input.data_type().equals_datatype(output.data_type())
            })
    {
        return Err(
            "Iceberg input column names, order and types must match the existing table".into(),
        );
    }
    Ok(())
}

pub(crate) fn validate_sink_batch_schema(
    schema: &arrow_schema::Schema,
    expected: &arrow_schema::Schema,
) -> Result<()> {
    if schema != expected {
        return Err("sink batch schema does not match the opened schema".into());
    }
    Ok(())
}

pub(crate) fn validate_iceberg_write_options(
    mode: &str,
    catalog_uri: &str,
    storage_root: &str,
) -> Result<()> {
    if mode != "append" {
        return Err("Iceberg COPY requires APPEND to an existing table; create and overwrite are unsupported".into());
    }
    if catalog_uri.is_empty() || storage_root.is_empty() {
        return Err("Iceberg COPY requires CATALOG_URI and STORAGE_ROOT".into());
    }
    Ok(())
}

/// Validate a configured write size; only Blob inline thresholds accept zero.
pub(crate) fn byte_size(name: &str, value: i64) -> Result<usize> {
    let inline_size = name.eq_ignore_ascii_case("blob_inline_size_threshold");
    if value < 0 || (!inline_size && value == 0) {
        let requirement = if inline_size {
            "non-negative"
        } else {
            "greater than zero"
        };
        return Err(format!("{name} must be {requirement}").into());
    }
    Ok(usize::try_from(value).map_err(|_| format!("{name} exceeds platform size"))?)
}

/// Validate an optional byte count; -1 preserves the backend default.
pub(crate) fn optional_byte_size(name: &str, value: i64) -> Result<Option<usize>> {
    if value == -1 {
        return Ok(None);
    }
    byte_size(name, value).map(Some)
}

pub(crate) fn sample_percent(value: f64) -> Result<()> {
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err("SAMPLE_PERCENT must be between 0 and 100".into());
    }
    Ok(())
}

pub(crate) fn sample_rows(value: i64, max_rows: u64) -> Result<()> {
    if value < 0 || value as u64 > max_rows {
        return Err(format!("SAMPLE_ROWS must be between 0 and {max_rows}").into());
    }
    Ok(())
}

pub(crate) fn encoding_path(schema: &arrow_schema::Schema, name: &str) -> Result<Vec<String>> {
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
            .ok_or_else(|| format!("Column encoding field not found: {name}"))?;
        path.push(field.name().clone());
        if parts.peek().is_none() {
            break;
        }
        let mut data_type = field.data_type();
        loop {
            use arrow_schema::DataType;
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
                _ => return Err(format!("Column encoding field not found: {name}").into()),
            }
        }
    }
    Ok(path)
}

pub(crate) fn column_compression(
    column: &str,
    codec: Option<&str>,
    level: Option<i32>,
) -> Result<Option<open_arrow_sink_core::CompressionSpec>> {
    use open_arrow_sink_core::CompressionSpec;
    let Some(codec) = codec else {
        if level.is_some() {
            return Err(format!(
                "COLUMN_COMPRESSION_LEVEL requires explicit zstd compression: {column}"
            )
            .into());
        }
        return Ok(None);
    };
    let compression = match codec.to_ascii_lowercase().as_str() {
        "none" | "uncompressed" => CompressionSpec::Uncompressed,
        "zstd" => CompressionSpec::Zstd { level },
        "lz4" | "lz4_raw" => CompressionSpec::Lz4Raw,
        "snappy" => CompressionSpec::Snappy,
        "gzip" => CompressionSpec::Gzip,
        _ => return Err(format!("Unknown COLUMN_COMPRESSION codec {codec}: {column}").into()),
    };
    if level.is_some() && !matches!(compression, CompressionSpec::Zstd { .. }) {
        return Err(
            format!("COLUMN_COMPRESSION_LEVEL is only supported for zstd: {column}").into(),
        );
    }
    Ok(Some(compression))
}

pub(crate) fn column_structural_encoding(
    column: &str,
    path: &[String],
    encoding: &str,
) -> Result<open_arrow_sink_core::StructuralEncoding> {
    use open_arrow_sink_core::StructuralEncoding;
    if path.len() != 1 {
        return Err(
            "COLUMN_STRUCTURAL_ENCODING applies to top-level columns, not nested fields".into(),
        );
    }
    match encoding.to_ascii_lowercase().as_str() {
        "miniblock" => Ok(StructuralEncoding::Miniblock),
        "fullzip" => Ok(StructuralEncoding::Fullzip),
        _ => {
            Err(format!("COLUMN_STRUCTURAL_ENCODING must be miniblock or fullzip: {column}").into())
        }
    }
}

pub(crate) fn index_spec(
    schema: &arrow_schema::Schema,
    column: &str,
    option: &str,
) -> Result<open_arrow_sink_core::IndexSpec> {
    use open_arrow_sink_core::{IndexSpec, IndexType};
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

pub(crate) fn validate_blob_indexes(
    indexes: &[open_arrow_sink_core::IndexSpec],
    blob_columns: &std::collections::HashSet<String>,
) -> Result<()> {
    if let Some(index) = indexes
        .iter()
        .find(|index| blob_columns.contains(&index.column))
    {
        return Err(format!(
            "BLOB_COLUMNS column cannot also be indexed: {}",
            index.column
        )
        .into());
    }
    Ok(())
}
