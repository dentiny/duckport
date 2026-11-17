//! Explicit nested BLOB_COLUMNS selection; downloads remain native sink work.
use crate::{error::Result, utils, validation};
use arrow_array::{
    Array, ArrayRef, StringArray, StructArray, cast::AsArray, make_array, new_null_array,
};
use arrow_buffer::NullBuffer;
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
        let path = utils::column_path(schema, &name, "BLOB_COLUMNS")?;
        let field = validation::required(
            open_arrow_sink_core::validation::nested_field(schema, &path),
            "BLOB_COLUMNS path not found",
        )?;
        validation::validate_blob_column(field, &name)?;
        roots.insert(path[0].clone());
        if path.len() == 1
            && matches!(
                field.data_type(),
                DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View
            )
        {
            top_level.insert(path[0].clone());
        } else if !paths.insert(path) {
            return Err(format!("Duplicate BLOB_COLUMNS path: {name}").into());
        }
    }
    validation::validate_blob_indexes(&options.indexes, &roots)?;
    let remaps = schema
        .fields()
        .iter()
        .filter_map(|source| {
            let target = Arc::new(output_field(
                source,
                &[source.name().clone()],
                &paths,
                options,
            ));
            if source == &target {
                return None;
            }
            let output = target.data_type().clone();
            Some(FieldRemap::new(
                source.name().clone(),
                target,
                move |array| {
                    transform(array, &output, None)
                        .map_err(|error| Error::InvalidInput(error.to_string()))
                },
            ))
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

fn transform(array: ArrayRef, target: &DataType, parent: Option<NullBuffer>) -> Result<ArrayRef> {
    if array.data_type() == target {
        return Ok(array);
    }
    let visible = NullBuffer::union(array.nulls(), parent.as_ref());
    if matches!(
        array.data_type(),
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View
    ) {
        return blob_uris(array, target, visible);
    }
    let data = array.to_data();
    let children = match target {
        DataType::Struct(fields) => fields
            .iter()
            .zip(data.child_data())
            .map(|(field, child)| {
                let child_type = field.data_type();
                transform(make_array(child.clone()), child_type, visible.clone())
                    .map(|array| array.to_data())
            })
            .collect::<Result<Vec<_>>>()?,
        DataType::List(field) | DataType::LargeList(field) | DataType::FixedSizeList(field, _) => {
            let offset = |row| match data.data_type() {
                DataType::List(_) => data.buffer::<i32>(0)[row] as usize,
                DataType::LargeList(_) => data.buffer::<i64>(0)[row] as usize,
                DataType::FixedSizeList(_, width) => (data.offset() + row) * *width as usize,
                _ => unreachable!("Blob parent must retain its list type"),
            };
            let child = &data.child_data()[0];
            let mut mask = vec![false; child.len()];
            for row in 0..array.len() {
                if visible.as_ref().is_none_or(|nulls| nulls.is_valid(row)) {
                    mask[offset(row)..offset(row + 1)].fill(true);
                }
            }
            let child_type = field.data_type();
            let child = transform(make_array(child.clone()), child_type, Some(mask.into()))?;
            vec![child.to_data()]
        }
        _ => unreachable!("Blob parent must be a struct or list"),
    };
    Ok(make_array(
        data.into_builder()
            .data_type(target.clone())
            .child_data(children)
            .build()?,
    ))
}

fn blob_uris(array: ArrayRef, target: &DataType, visible: Option<NullBuffer>) -> Result<ArrayRef> {
    let array = arrow_cast::cast(&array, &DataType::Utf8)?;
    let paths = array.as_string::<i32>();
    let uris = StringArray::new(
        paths.offsets().clone(),
        paths.values().clone(),
        visible.clone(),
    );
    let DataType::Struct(fields) = target else {
        unreachable!("Blob target is a struct")
    };
    Ok(Arc::new(StructArray::try_new(
        fields.clone(),
        vec![
            new_null_array(&DataType::LargeBinary, array.len()),
            Arc::new(uris),
        ],
        visible,
    )?))
}
