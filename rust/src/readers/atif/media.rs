//! Normalize ATIF media references without reading their payloads.
use super::{SOURCE_TYPE, STORAGE_POLICY, StoragePolicy};
use crate::error::Result;
use arrow_array::{
    ArrayRef, RecordBatch, RecordBatchOptions, builder::StringBuilder, cast::AsArray, make_array,
};
use arrow_schema::{DataType, FieldRef, SchemaRef};
use open_arrow_access::{RecordReader, map_record_reader};
use std::{path::Path, sync::Arc};
use url::Url;

pub(crate) fn uri_schema(schema: &SchemaRef, document: &str) -> SchemaRef {
    fn tag_field(field: &FieldRef, path: &str, document: &str) -> FieldRef {
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
            DataType::Struct(fields) => DataType::Struct(
                fields
                    .iter()
                    .map(|child| tag_field(child, &format!("{path}.{}", child.name()), document))
                    .collect(),
            ),
            DataType::List(item) => DataType::List(tag_field(item, path, document)),
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
            if policy == StoragePolicy::Blob {
                field
                    .metadata_mut()
                    .insert("duckport:media_document".into(), document.into());
            }
        }
        Arc::new(field)
    }
    let mut schema = schema.as_ref().clone();
    schema.fields = schema
        .fields
        .iter()
        .map(|field| tag_field(field, field.name(), document))
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
        return Ok(Url::from_file_path(reference)
            .map_err(|_| "invalid ATIF media path")?
            .to_string());
    }
    if let Ok(url) = Url::parse(reference) {
        if url.scheme() == "file" {
            url.to_file_path()
                .map_err(|_| "invalid ATIF media file URI")?;
            return Ok(url.to_string());
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
    let path = Path::new(document)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(reference);
    Ok(Url::from_file_path(path)
        .map_err(|_| "invalid ATIF media path")?
        .to_string())
}

pub(crate) struct MediaResolver {
    pub document: String,
}
impl MediaResolver {
    pub(crate) fn schema(&self, schema: &SchemaRef) -> SchemaRef {
        uri_schema(schema, &self.document)
    }
    pub(crate) fn reader(self: &Arc<Self>, reader: Box<dyn RecordReader>) -> Box<dyn RecordReader> {
        let capability = reader.capability();
        let schema = self.schema(&reader.metadata().schema);
        let resolver = self.clone();
        Box::new(map_record_reader(
            reader,
            schema.clone(),
            capability,
            move |batch| {
                resolver.resolve(batch, schema.clone()).map_err(|error| {
                    open_arrow_access::Error::new(
                        open_arrow_access::ErrorKind::Decode,
                        "ATIF media URI",
                        error.to_string(),
                    )
                })
            },
        ))
    }
    fn resolve(&self, batch: RecordBatch, schema: SchemaRef) -> Result<RecordBatch> {
        let columns = batch
            .columns()
            .iter()
            .zip(schema.fields())
            .map(|(array, field)| self.resolve_array(array, field))
            .collect::<Result<Vec<_>>>()?;
        Ok(RecordBatch::try_new_with_options(
            schema,
            columns,
            &RecordBatchOptions::new().with_row_count(Some(batch.num_rows())),
        )?)
    }
    fn resolve_array(&self, array: &ArrayRef, field: &FieldRef) -> Result<ArrayRef> {
        if field
            .metadata()
            .get(STORAGE_POLICY)
            .is_some_and(|p| p == "blob")
        {
            let paths = array.as_string::<i32>();
            let mut uris = StringBuilder::new();
            for path in paths.iter() {
                match path {
                    Some(path) => uris.append_value(resolve_uri(&self.document, path)?),
                    None => uris.append_null(),
                }
            }
            return Ok(Arc::new(uris.finish()));
        }
        let children: &[FieldRef] = match field.data_type() {
            DataType::Struct(fields) => fields,
            DataType::List(item) => std::slice::from_ref(item),
            _ => return Ok(array.clone()),
        };
        let data = array.to_data();
        let transformed = data
            .child_data()
            .iter()
            .zip(children)
            .map(|(child, field)| {
                self.resolve_array(&make_array(child.clone()), field)
                    .map(|array| array.to_data())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(make_array(
            data.into_builder()
                .data_type(field.data_type().clone())
                .child_data(transformed)
                .build()?,
        ))
    }
}
