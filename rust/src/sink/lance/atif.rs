//! ATIF policy only; schema adaptation and media I/O belong to sink layers.
use crate::{error::Result, readers::atif::STORAGE_POLICY, utils::map_field_tree};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use std::{collections::HashSet, sync::Arc};

// Collect tagged URL paths and promote binary text with the default codec.
fn apply_storage_policy(mut field: Field, path: &str, urls: &mut HashSet<String>) -> Result<Field> {
    match field.metadata().get(STORAGE_POLICY).map(String::as_str) {
        Some("blob") if field.data_type().is_string() => {
            urls.insert(path.to_owned());
        }
        Some("text") if matches!(field.data_type(), DataType::Binary | DataType::LargeBinary) => {
            field = field.with_data_type(DataType::LargeBinary);
            // Default ATIF text compression to lz4, preserving any existing codec.
            field
                .metadata_mut()
                .entry("lance-encoding:compression".into())
                .or_insert_with(|| "lz4".into());
        }
        Some("blob" | "text") | None => {}
        Some(policy) => return Err(format!("invalid ATIF storage policy: {policy}").into()),
    }
    Ok(field)
}

// Return the target schema for schema evolution and media paths for DownloadBlobLayer.
// Preserve schema metadata; the sink layers perform the actual casts and downloads.
pub(super) fn configure(schema: &SchemaRef) -> Result<(SchemaRef, HashSet<String>)> {
    let mut urls = HashSet::new();
    let fields = schema
        .fields()
        .iter()
        .map(|field| {
            map_field_tree(field, field.name(), &mut |field, path| {
                apply_storage_policy(field, path, &mut urls)
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((
        Arc::new(Schema::new_with_metadata(fields, schema.metadata().clone())),
        urls,
    ))
}
