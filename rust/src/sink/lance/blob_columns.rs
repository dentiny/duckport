//! Explicit nested BLOB_COLUMNS selection; downloads remain native sink work.
use crate::{error::Result, utils, validation};
use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, LargeListArray, ListArray, StringArray, StructArray,
    cast::AsArray, new_null_array,
};
use arrow_buffer::NullBuffer;
use arrow_schema::{DataType, Field, Schema};
use lance::{BlobFieldOptions, blob_field_with_options};
use open_arrow_sink_core::{Error, WriteOptions};
use open_arrow_sink_layer::{FieldRemap, SchemaEvolutionLayer};
use std::{collections::HashSet, sync::Arc};

pub(super) struct BlobColumns {
    pub top_level: HashSet<String>,
    pub roots: HashSet<String>,
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
            let source = source.clone();
            let output = target.clone();
            Some(FieldRemap::new(
                source.name().clone(),
                target,
                move |array| {
                    transform(array, &source, &output, None)
                        .map_err(|error| Error::InvalidInput(error.to_string()))
                },
            ))
        })
        .collect();
    Ok(BlobColumns {
        top_level,
        roots,
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

fn transform(
    array: ArrayRef,
    source: &Field,
    target: &Field,
    parent: Option<NullBuffer>,
) -> Result<ArrayRef> {
    if source == target {
        return Ok(array);
    }
    let visible = NullBuffer::union(array.nulls(), parent.as_ref());
    if matches!(
        source.data_type(),
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View
    ) {
        return blob_uris(array, target, visible);
    }
    match (source.data_type(), target.data_type()) {
        (DataType::Struct(input), DataType::Struct(output)) => {
            let values = array.as_struct();
            let children = values
                .columns()
                .iter()
                .zip(input)
                .zip(output)
                .map(|((array, input), output)| {
                    transform(array.clone(), input, output, visible.clone())
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Arc::new(StructArray::try_new(
                output.clone(),
                children,
                values.nulls().cloned(),
            )?))
        }
        (DataType::List(input), DataType::List(output)) => {
            let list = array.as_list::<i32>();
            let mask = list_mask(
                list.values().len(),
                list.value_offsets().iter().map(|v| *v as usize).collect(),
                visible.as_ref(),
            );
            let values = transform(list.values().clone(), input, output, mask)?;
            Ok(Arc::new(ListArray::try_new(
                output.clone(),
                list.offsets().clone(),
                values,
                list.nulls().cloned(),
            )?))
        }
        (DataType::LargeList(input), DataType::LargeList(output)) => {
            let list = array.as_list::<i64>();
            let mask = list_mask(
                list.values().len(),
                list.value_offsets().iter().map(|v| *v as usize).collect(),
                visible.as_ref(),
            );
            let values = transform(list.values().clone(), input, output, mask)?;
            Ok(Arc::new(LargeListArray::try_new(
                output.clone(),
                list.offsets().clone(),
                values,
                list.nulls().cloned(),
            )?))
        }
        (DataType::FixedSizeList(input, width), DataType::FixedSizeList(output, _)) => {
            let list = array.as_fixed_size_list();
            let offsets = (0..=list.len()).map(|i| i * *width as usize).collect();
            let mask = list_mask(list.values().len(), offsets, visible.as_ref());
            let values = transform(list.values().clone(), input, output, mask)?;
            Ok(Arc::new(FixedSizeListArray::try_new_with_length(
                output.clone(),
                *width,
                values,
                list.nulls().cloned(),
                list.len(),
            )?))
        }
        _ => Ok(arrow_cast::cast(&array, target.data_type())?),
    }
}

fn list_mask(len: usize, offsets: Vec<usize>, visible: Option<&NullBuffer>) -> Option<NullBuffer> {
    let mut mask = vec![false; len];
    for (row, range) in offsets.windows(2).enumerate() {
        if visible.is_none_or(|nulls| nulls.is_valid(row)) {
            mask[range[0]..range[1]].fill(true);
        }
    }
    Some(NullBuffer::from(mask))
}

fn blob_uris(array: ArrayRef, target: &Field, visible: Option<NullBuffer>) -> Result<ArrayRef> {
    let array = arrow_cast::cast(&array, &DataType::Utf8)?;
    let paths = array.as_string::<i32>();
    let uris = StringArray::new(
        paths.offsets().clone(),
        paths.values().clone(),
        visible.clone(),
    );
    let DataType::Struct(fields) = target.data_type() else {
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
