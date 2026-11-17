//! Normalize ATIF media references without reading their payloads.
use super::STORAGE_POLICY;
use crate::error::Result;
use arrow_array::{
    ArrayRef, RecordBatch, RecordBatchOptions, builder::StringBuilder, cast::AsArray, make_array,
};
use arrow_schema::{DataType, FieldRef, SchemaRef};
use open_arrow_access::{RecordReader, map_record_reader};
use std::sync::Arc;
use url::Url;

fn tag_field(field: &FieldRef, path: &str, document: &str) -> FieldRef {
    let policy = match path {
        "steps.message.text" | "steps.observation.results.content.text" => Some("text"),
        "steps.message.source.path" | "steps.observation.results.content.source.path" => {
            Some("blob")
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
            .insert(STORAGE_POLICY.into(), policy.into());
        if policy == "blob" {
            field
                .metadata_mut()
                .insert("duckport:media_document".into(), document.into());
        }
    }
    Arc::new(field)
}

pub(crate) fn uri_schema(schema: &SchemaRef, document: &str) -> SchemaRef {
    let mut schema = schema.as_ref().clone();
    schema.fields = schema
        .fields
        .iter()
        .map(|field| tag_field(field, field.name(), document))
        .collect();
    Arc::new(schema)
}

/// Interpret media references uniformly as URI references. Filesystem access and
/// transport support belong to storage/backends, not the ATIF format adapter.
fn resolve_uri(document: &Url, reference: &str) -> Result<String> {
    let target = document.join(reference)?;
    // Inline content is a representation, not a remote storage transport.
    if target.scheme() == "data" {
        return Ok(reference.to_owned());
    }
    if target.cannot_be_a_base() {
        return Err("ATIF media URI must be hierarchical".into());
    }
    if !target.username().is_empty() || target.password().is_some() {
        return Err("ATIF media URI must not contain embedded credentials".into());
    }
    if document.scheme() != "file" && !crate::storage::same_storage_origin(document, &target) {
        return Err("ATIF remote media must use the trajectory's storage origin".into());
    }
    Ok(target.to_string())
}

pub(crate) struct MediaResolver {
    pub document: Url,
}
impl MediaResolver {
    pub(crate) fn schema(&self, schema: &SchemaRef) -> SchemaRef {
        uri_schema(schema, self.document.as_str())
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
    // Resolve media references in each projected column using the tagged schema.
    // Preserve the row count explicitly for zero-column scans such as COUNT(*).
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
    // Normalize tagged media paths to absolute URI strings without fetching payloads.
    // Recurse through structs/lists, retaining their offsets and null buffers;
    // reuse untagged leaf arrays and preserve null media paths.
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
