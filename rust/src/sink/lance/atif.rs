//! ATIF policy only; schema adaptation and media I/O belong to sink layers.
use crate::error::Result;
use arrow_schema::{DataType, FieldRef, Schema, SchemaRef};
use std::{collections::HashSet, sync::Arc};

fn field(input: &FieldRef, path: &str, urls: &mut HashSet<String>) -> Result<FieldRef> {
    let mut output = input.as_ref().clone();
    if input
        .metadata()
        .get("duckport:source_type")
        .is_some_and(|v| v == "atif")
    {
        match input
            .metadata()
            .get("duckport:storage_policy")
            .map(String::as_str)
        {
            Some("blob") if input.data_type().is_string() => {
                urls.insert(path.to_owned());
            }
            Some("text")
                if matches!(input.data_type(), DataType::Binary | DataType::LargeBinary) =>
            {
                output = output.with_data_type(DataType::LargeBinary);
                output
                    .metadata_mut()
                    .entry("lance-encoding:compression".into())
                    .or_insert_with(|| "lz4".into());
            }
            Some("blob" | "text") | None => {}
            Some(policy) => return Err(format!("invalid ATIF storage policy: {policy}").into()),
        }
    }
    output = output.clone().with_data_type(match output.data_type() {
        DataType::Struct(fields) => DataType::Struct(
            fields
                .iter()
                .map(|child| field(child, &format!("{path}.{}", child.name()), urls))
                .collect::<Result<_>>()?,
        ),
        DataType::List(item) => DataType::List(field(item, path, urls)?),
        DataType::LargeList(item) => DataType::LargeList(field(item, path, urls)?),
        other => other.clone(),
    });
    Ok(Arc::new(output))
}

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
