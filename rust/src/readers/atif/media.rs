//! Materialize media at its native ATIF field position before writing Lance.

use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, RecordBatch, RecordBatchOptions, StructArray, builder::LargeBinaryBuilder,
    cast::AsArray, make_array,
};
use arrow_schema::{DataType, FieldRef, SchemaRef};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::{StreamExt, future::BoxFuture};
use open_arrow_access::{RecordReader, StreamReader, into_stream};
use url::Url;

use super::{ContentType, SOURCE_TYPE, STORAGE_POLICY, StoragePolicy};
use crate::{error::Result, options::StorageConfig, storage};

/// Each Arrow struct row is one ContentPart with its own concrete type.
fn content_type(part: &StructArray, index: usize) -> Result<ContentType> {
    match part["type"].as_string::<i32>().value(index) {
        "text" => Ok(ContentType::Text),
        "image" => Ok(ContentType::Image),
        "audio" => Ok(ContentType::Audio),
        kind => Err(std::io::Error::other(format!("unsupported ATIF content type: {kind}")).into()),
    }
}

/// Materialize MediaSource.path in-place. Field metadata describes only storage;
/// each ContentPart.type retains the actual text/image/audio variant.
pub(crate) fn embedded_schema(schema: &SchemaRef) -> SchemaRef {
    fn embedded_field(field: &FieldRef, path: &str) -> FieldRef {
        let policy = match path {
            "steps.message.text" | "steps.observation.results.content.text" => {
                Some(StoragePolicy::Text)
            }
            "steps.message.source.path" | "steps.observation.results.content.source.path" => {
                Some(StoragePolicy::Blob)
            }
            _ => None,
        };
        let data_type = match field.data_type() {
            _ if policy == Some(StoragePolicy::Blob) => DataType::LargeBinary,
            DataType::Struct(fields) => DataType::Struct(
                fields
                    .iter()
                    .map(|child| embedded_field(child, &format!("{path}.{}", child.name())))
                    .collect(),
            ),
            DataType::List(item) => DataType::List(embedded_field(item, path)),
            other => other.clone(),
        };
        let mut field = field.as_ref().clone().with_data_type(data_type);
        if let Some(policy) = policy {
            field
                .metadata_mut()
                .insert(SOURCE_TYPE.into(), "atif".into());
            field
                .metadata_mut()
                .insert(STORAGE_POLICY.into(), policy.as_str().into());
        }
        Arc::new(field)
    }
    let mut schema = schema.as_ref().clone();
    schema.fields = schema
        .fields
        .iter()
        .map(|field| embedded_field(field, field.name()))
        .collect();
    schema.metadata.insert(SOURCE_TYPE.into(), "atif".into());
    Arc::new(schema)
}

/// Remote documents cannot cause local-file reads or forward S3 credentials to
/// another bucket. Relative remote paths use URL resolution, not host paths.
fn resolve_uri(document: &str, reference: &str) -> Result<String> {
    if reference.starts_with("data:") {
        return Ok(reference.to_owned());
    }
    if let Ok(base) = Url::parse(document)
        && matches!(base.scheme(), "s3" | "http" | "https")
    {
        let target = base.join(reference)?;
        if target.scheme() != base.scheme()
            || target.host_str() != base.host_str()
            || target.port_or_known_default() != base.port_or_known_default()
            || !target.username().is_empty()
            || target.password().is_some()
        {
            return Err(std::io::Error::other(
                "ATIF remote media must use the trajectory's storage origin",
            )
            .into());
        }
        return Ok(target.to_string());
    }
    if Path::new(reference).is_absolute() {
        return Ok(reference.to_owned());
    }
    if let Ok(url) = Url::parse(reference) {
        if url.scheme() == "file" {
            return url
                .to_file_path()
                .map(|path| path.to_string_lossy().into_owned())
                .map_err(|_| "invalid ATIF media file URI".into());
        }
        if matches!(url.scheme(), "s3" | "http" | "https") {
            return Ok(url.to_string());
        }
        return Err(std::io::Error::other(format!(
            "unsupported ATIF media scheme: {}",
            url.scheme()
        ))
        .into());
    }
    Ok(Path::new(document)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(reference)
        .to_string_lossy()
        .into_owned())
}

fn decode_data_uri(uri: &str) -> Result<Vec<u8>> {
    let (header, encoded) = uri
        .split_once(',')
        .ok_or_else(|| std::io::Error::other("invalid ATIF media data URI"))?;
    let encoded = percent_encoding::percent_decode_str(encoded).collect::<Vec<_>>();
    if header.ends_with(";base64") {
        STANDARD
            .decode(encoded)
            .map_err(|error| format!("invalid ATIF media base64: {error}").into())
    } else {
        Ok(encoded)
    }
}

pub(crate) struct MediaResolver {
    pub document: String,
    pub config: StorageConfig,
}

impl MediaResolver {
    /// Keep async materialization lazy while the library owns EOF/error cleanup
    /// and attaches the trajectory source to errors. No prefetch queue is needed.
    pub(crate) fn reader(self: &Arc<Self>, reader: Box<dyn RecordReader>) -> Box<dyn RecordReader> {
        let capability = reader.capability();
        let mut metadata = reader.metadata().clone();
        metadata.schema = embedded_schema(&metadata.schema);
        let schema = metadata.schema.clone();
        let resolver = self.clone();
        let batches = into_stream(reader).then(move |batch| {
            let resolver = resolver.clone();
            let schema = schema.clone();
            async move {
                resolver.resolve(batch?, schema).await.map_err(|error| {
                    open_arrow_access::Error::new(
                        open_arrow_access::ErrorKind::Decode,
                        "ATIF media",
                        error.to_string(),
                    )
                })
            }
        });
        Box::new(StreamReader::records_with_capability(
            batches, metadata, capability,
        ))
    }

    async fn read(&self, uri: &str) -> Result<Vec<u8>> {
        if uri.starts_with("data:") {
            return decode_data_uri(uri);
        }
        let (operator, path) = storage::operator(uri, &self.config)?;
        Ok(operator.read(&path).await?.to_vec())
    }

    /// Materialize message/observation media. Projections excluding steps skip this resolver.
    pub async fn resolve(&self, batch: RecordBatch, schema: SchemaRef) -> Result<RecordBatch> {
        let mut columns = Vec::with_capacity(batch.num_columns());
        for (array, field) in batch.columns().iter().zip(schema.fields()) {
            columns.push(self.resolve_array(array, field, None).await?);
        }
        Ok(RecordBatch::try_new_with_options(
            schema,
            columns,
            &RecordBatchOptions::new().with_row_count(Some(batch.num_rows())),
        )?)
    }

    fn resolve_array<'a>(
        &'a self,
        array: &'a ArrayRef,
        target: &'a FieldRef,
        part: Option<&'a StructArray>,
    ) -> BoxFuture<'a, Result<ArrayRef>> {
        Box::pin(async move {
            if array.data_type() == target.data_type() {
                return Ok(array.clone());
            }
            // The native schema marks ContentPart.text explicitly; no shape inference.
            let part = match target.data_type() {
                DataType::Struct(fields)
                    if fields.iter().any(|field| {
                        field
                            .metadata()
                            .get(STORAGE_POLICY)
                            .is_some_and(|policy| policy == "text")
                    }) =>
                {
                    Some(array.as_struct())
                }
                _ => part,
            };
            if array.data_type() == &DataType::Utf8 && target.data_type() == &DataType::LargeBinary
            {
                let part = part.ok_or_else(|| {
                    std::io::Error::other("media path outside an ATIF ContentPart")
                })?;
                let paths = array.as_string::<i32>();
                let mut bytes = LargeBinaryBuilder::new();
                for index in 0..part.len() {
                    if part.is_null(index) {
                        bytes.append_null();
                        continue;
                    }
                    match content_type(part, index)? {
                        ContentType::Text => bytes.append_null(),
                        ContentType::Image | ContentType::Audio => {
                            if part["source"].is_null(index) || paths.is_null(index) {
                                return Err("missing ATIF media source path".into());
                            }
                            let path = paths.value(index);
                            let uri = resolve_uri(&self.document, path)?;
                            let payload = self
                                .read(&uri)
                                .await
                                .map_err(|error| format!("ATIF media {path}: {error}"))?;
                            bytes.append_value(payload);
                        }
                    }
                }
                return Ok(Arc::new(bytes.finish()) as ArrayRef);
            }
            let children: &[FieldRef] = match target.data_type() {
                DataType::Struct(fields) => fields,
                DataType::List(item) => std::slice::from_ref(item),
                _ => return Err("unexpected ATIF media field type".into()),
            };
            let data = array.to_data();
            let mut transformed = Vec::with_capacity(children.len());
            for (child, field) in data.child_data().iter().zip(children) {
                let child = make_array(child.clone());
                transformed.push(self.resolve_array(&child, field, part).await?.to_data());
            }
            Ok(make_array(
                data.into_builder()
                    .data_type(target.data_type().clone())
                    .child_data(transformed)
                    .build()?,
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_is_per_content_part() {
        let parts = StructArray::from(vec![(
            Arc::new(arrow_schema::Field::new("type", DataType::Utf8, false)),
            Arc::new(arrow_array::StringArray::from(vec![
                "text", "image", "audio", "video",
            ])) as ArrayRef,
        )]);
        assert_eq!(content_type(&parts, 0).unwrap(), ContentType::Text);
        assert_eq!(content_type(&parts, 1).unwrap(), ContentType::Image);
        assert_eq!(content_type(&parts, 2).unwrap(), ContentType::Audio);
        assert!(
            content_type(&parts, 3)
                .unwrap_err()
                .to_string()
                .contains("unsupported ATIF content type: video")
        );
    }

    #[test]
    fn resolves_media_relative_to_the_document() {
        assert_eq!(
            resolve_uri("/runs/a/trajectory.json", "images/a.png").unwrap(),
            Path::new("/runs/a").join("images/a.png").to_string_lossy()
        );
        assert_eq!(
            resolve_uri("https://example.com/run/t.json", "../image.png").unwrap(),
            "https://example.com/image.png"
        );
        assert_eq!(
            resolve_uri("s3://bucket/run/t.json.gz", "a.png").unwrap(),
            "s3://bucket/run/a.png"
        );
        for path in [
            "file:///etc/passwd",
            "s3://other/image.png",
            "https://other/image.png",
        ] {
            assert!(resolve_uri("s3://bucket/run/t.json", path).is_err());
        }
    }

    #[test]
    fn decodes_data_uris() {
        assert_eq!(
            decode_data_uri("data:image/png;base64,AP8=").unwrap(),
            [0, 255]
        );
        assert_eq!(decode_data_uri("data:image/png,%00%ff").unwrap(), [0, 255]);
        assert_eq!(
            decode_data_uri("data:image/png;base64,AP8%3D").unwrap(),
            [0, 255]
        );
        assert!(decode_data_uri("data:image/png").is_err());
        assert!(decode_data_uri("data:image/png;base64,????").is_err());
    }
}
