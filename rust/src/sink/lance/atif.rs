//! Interpret ATIF storage metadata without inferring semantics from column names.
use crate::error::Result;
use arrow_schema::{DataType, FieldRef, Schema, SchemaRef};
use open_arrow_sink_core::{BLOB_METADATA_KEY, BLOB_METADATA_VALUE};
use std::sync::Arc;

pub(super) fn configure(schema: &SchemaRef) -> Result<SchemaRef> {
    fn field(input: &FieldRef) -> Result<FieldRef> {
        let mut output = input.as_ref().clone();
        if input
            .metadata()
            .get("duckport:source_type")
            .is_some_and(|v| v == "atif")
            && matches!(input.data_type(), DataType::Binary | DataType::LargeBinary)
        {
            match input
                .metadata()
                .get("duckport:storage_policy")
                .map(String::as_str)
            {
                Some("blob") => {
                    output = output.with_data_type(DataType::LargeBinary);
                    output
                        .metadata_mut()
                        .insert(BLOB_METADATA_KEY.into(), BLOB_METADATA_VALUE.into());
                }
                Some("text") => {
                    output = output.with_data_type(DataType::LargeBinary);
                    output
                        .metadata_mut()
                        .entry("lance-encoding:compression".into())
                        .or_insert_with(|| "lz4".into());
                }
                None => {}
                Some(policy) => return Err(format!("invalid ATIF storage policy: {policy}").into()),
            }
        } else {
            output = output.with_data_type(match input.data_type() {
                DataType::Struct(fields) => {
                    DataType::Struct(fields.iter().map(field).collect::<Result<_>>()?)
                }
                DataType::List(item) => DataType::List(field(item)?),
                DataType::LargeList(item) => DataType::LargeList(field(item)?),
                other => other.clone(),
            });
        }
        Ok(Arc::new(output))
    }
    Ok(Arc::new(Schema::new_with_metadata(
        schema
            .fields()
            .iter()
            .map(field)
            .collect::<Result<Vec<_>>>()?,
        schema.metadata().clone(),
    )))
}
