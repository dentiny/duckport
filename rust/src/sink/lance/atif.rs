//! ATIF policy only; schema adaptation and media I/O belong to sink layers.
use crate::{error::Result, readers::atif::STORAGE_POLICY};
use arrow_schema::{DataType, FieldRef, Schema, SchemaRef};
use std::{collections::HashSet, sync::Arc};

// Apply ATIF storage hints recursively, collecting tagged string paths for download
// and promoting tagged binary text to large binary with lz4 as the default codec.
// List elements keep their parent's path because sink paths omit list item names.
fn field(input: &FieldRef, path: &str, urls: &mut HashSet<String>) -> Result<FieldRef> {
    let data_type = match input.data_type() {
        DataType::Struct(fields) => DataType::Struct(
            fields
                .iter()
                .map(|child| field(child, &format!("{path}.{}", child.name()), urls))
                .collect::<Result<_>>()?,
        ),
        DataType::List(item) => DataType::List(field(item, path, urls)?),
        DataType::LargeList(item) => DataType::LargeList(field(item, path, urls)?),
        other => other.clone(),
    };
    let mut output = input.as_ref().clone().with_data_type(data_type);
    match input.metadata().get(STORAGE_POLICY).map(String::as_str) {
        Some("blob") if input.data_type().is_string() => {
            urls.insert(path.to_owned());
        }
        Some("text") if matches!(input.data_type(), DataType::Binary | DataType::LargeBinary) => {
            output = output.with_data_type(DataType::LargeBinary);
            // Default ATIF text compression to lz4, preserving any existing codec.
            output
                .metadata_mut()
                .entry("lance-encoding:compression".into())
                .or_insert_with(|| "lz4".into());
        }
        Some("blob" | "text") | None => {}
        Some(policy) => return Err(format!("invalid ATIF storage policy: {policy}").into()),
    }
    Ok(Arc::new(output))
}

// Return the target schema for schema evolution and media paths for DownloadBlobLayer.
// Preserve schema metadata; the sink layers perform the actual casts and downloads.
pub(super) fn configure(schema: &SchemaRef) -> Result<(SchemaRef, HashSet<String>)> {
    let mut urls = HashSet::new();
    let fields = schema
        .fields()
        .iter()
        .map(|f| field(f, f.name(), &mut urls))
        .collect::<Result<Vec<_>>>()?;
    Ok((
        Arc::new(Schema::new_with_metadata(fields, schema.metadata().clone())),
        urls,
    ))
}
