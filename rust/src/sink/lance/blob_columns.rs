//! Explicit nested BLOB_COLUMNS selection; downloads remain native sink work.
use crate::{error::Result, utils, validation};
use arrow_array::{ArrayRef, StructArray, make_array, new_null_array};
use arrow_schema::{DataType, Field, Schema};
use lance::{BlobFieldOptions, blob_field_with_options};
use open_arrow_sink_core::{Error, WriteOptions};
use open_arrow_sink_layer::{FieldRemap, SchemaEvolutionLayer};
use std::{collections::HashSet, sync::Arc};

pub(super) struct BlobColumns {
    pub top_level: HashSet<String>,
    pub nested: SchemaEvolutionLayer,
}

pub(super) fn parse(
    schema: &Schema,
    columns: HashSet<String>,
    options: &WriteOptions,
) -> Result<BlobColumns> {
    let mut top_level = HashSet::new();
    let mut roots = HashSet::new();
    let mut paths = HashSet::new();
    for name in columns {
        let path = utils::resolve_column_path(schema, &name, "BLOB_COLUMNS")?;
        let field = validation::required(
            open_arrow_sink_core::validation::nested_field(schema, &path),
            "BLOB_COLUMNS path not found",
        )?;
        validation::validate_blob_column(field, &name)?;
        roots.insert(path[0].clone());
        if path.len() == 1 && field.data_type().is_string() {
            top_level.insert(path[0].clone());
        } else if !paths.insert(path) {
            return Err(format!("Duplicate BLOB_COLUMNS path: {name}").into());
        }
    }
    validation::validate_blob_indexes(&options.indexes, &roots)?;
    let remaps = schema
        .fields()
        .iter()
        .filter(|source| paths.iter().any(|path| path[0].eq(source.name())))
        .map(|source| {
            let target = output_field(source, &[source.name().clone()], &paths, options);
            let output = target.data_type().clone();
            FieldRemap::new(source.name().clone(), target, move |array| {
                transform(array, &output).map_err(|error| Error::InvalidInput(error.to_string()))
            })
        })
        .collect();
    Ok(BlobColumns {
        top_level,
        nested: SchemaEvolutionLayer::new(remaps),
    })
}

fn output_field(
    field: &Field,
    path: &[String],
    selected: &HashSet<Vec<String>>,
    options: &WriteOptions,
) -> Field {
    let data_type = match field.data_type() {
        DataType::List(child) => {
            DataType::List(Arc::new(output_field(child, path, selected, options)))
        }
        DataType::LargeList(child) => {
            DataType::LargeList(Arc::new(output_field(child, path, selected, options)))
        }
        DataType::FixedSizeList(child, width) => DataType::FixedSizeList(
            Arc::new(output_field(child, path, selected, options)),
            *width,
        ),
        _ if selected.contains(path) => {
            let mut layout = BlobFieldOptions::default();
            if let Some(blob) = &options.blob_handle {
                layout.inline_size_threshold = blob.inline_size_threshold;
                layout.dedicated_size_threshold = blob.dedicated_size_threshold;
            }
            let blob = blob_field_with_options(field.name(), true, layout);
            let mut metadata = field.metadata().clone();
            metadata.extend(blob.metadata().clone());
            return blob.with_metadata(metadata);
        }
        DataType::Struct(fields) => DataType::Struct(
            fields
                .iter()
                .map(|child| {
                    let mut path = path.to_vec();
                    path.push(child.name().clone());
                    Arc::new(output_field(child, &path, selected, options))
                })
                .collect(),
        ),
        _ => return field.clone(),
    };
    field.clone().with_data_type(data_type)
}

fn transform(array: ArrayRef, target: &DataType) -> Result<ArrayRef> {
    if array.data_type() == target {
        return Ok(array);
    }
    if array.data_type().is_string() {
        return blob_uris(array, target);
    }
    let fields: &[Arc<Field>] = match target {
        DataType::Struct(fields) => fields,
        DataType::List(field) | DataType::LargeList(field) | DataType::FixedSizeList(field, _) => {
            std::slice::from_ref(field)
        }
        _ => unreachable!("Blob parent must be a struct or list"),
    };
    let data = array.to_data();
    let children = fields
        .iter()
        .zip(data.child_data())
        .map(|(field, child)| {
            transform(make_array(child.clone()), field.data_type()).map(|array| array.to_data())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(make_array(
        data.into_builder()
            .data_type(target.clone())
            .child_data(children)
            .build()?,
    ))
}

fn blob_uris(array: ArrayRef, target: &DataType) -> Result<ArrayRef> {
    let array = arrow_cast::cast(&array, &DataType::Utf8)?;
    let len = array.len();
    let nulls = array.nulls().cloned();
    let DataType::Struct(fields) = target else {
        unreachable!("Blob target is a struct")
    };
    Ok(Arc::new(StructArray::try_new(
        fields.clone(),
        vec![new_null_array(&DataType::LargeBinary, len), array],
        nulls,
    )?))
}
