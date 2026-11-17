//! ATIF storage policy and sink-side adaptation to native nested Blob inputs.
use crate::{error::Result, utils, validation};
use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, LargeListArray, ListArray, StructArray,
    builder::{LargeBinaryBuilder, StringBuilder},
    cast::AsArray,
};
use arrow_buffer::NullBuffer;
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use base64::{Engine, engine::general_purpose::STANDARD};
use lance::{BlobFieldOptions, blob_field_with_options};
use open_arrow_sink_core::WriteOptions;
use open_arrow_sink_layer::{FieldRemap, SchemaEvolutionLayer};
use std::sync::Arc;
use url::Url;

pub(crate) const POLICY: &str = "duckport:atif_storage";
pub(crate) const SOURCE: &str = "duckport:atif_source";

pub(crate) fn is_media(field: &Field) -> bool {
    field.metadata().get(POLICY).map(String::as_str) == Some("media")
}

pub(crate) fn source_schema(schema: &Schema, locator: &str) -> Result<SchemaRef> {
    let source = utils::source_url(locator)?.to_string();
    let fields = schema
        .fields()
        .iter()
        .map(|field| tag_field(field, field.name(), &source))
        .collect::<Vec<_>>();
    Ok(Arc::new(Schema::new_with_metadata(
        fields,
        schema.metadata().clone(),
    )))
}

fn tag_field(field: &Field, path: &str, source: &str) -> Field {
    let data_type = map_children(field.data_type(), &|child, is_item| {
        let path = if is_item {
            path.to_string()
        } else {
            format!("{path}.{}", child.name())
        };
        tag_field(child, &path, source)
    });
    let mut field = field.clone().with_data_type(data_type);
    let content = ["steps.message", "steps.observation.results.content"];
    if content
        .iter()
        .any(|prefix| path == format!("{prefix}.text"))
    {
        field.metadata_mut().insert(POLICY.into(), "text".into());
    } else if content
        .iter()
        .any(|prefix| path == format!("{prefix}.source.path"))
    {
        field.metadata_mut().insert(POLICY.into(), "media".into());
        field.metadata_mut().insert(SOURCE.into(), source.into());
    }
    field
}

fn map_children(data_type: &DataType, map: &impl Fn(&Field, bool) -> Field) -> DataType {
    match data_type {
        DataType::Struct(fields) => DataType::Struct(
            fields
                .iter()
                .map(|field| Arc::new(map(field, false)))
                .collect(),
        ),
        DataType::List(child) => DataType::List(Arc::new(map(child, true))),
        DataType::LargeList(child) => DataType::LargeList(Arc::new(map(child, true))),
        DataType::FixedSizeList(child, width) => {
            DataType::FixedSizeList(Arc::new(map(child, true)), *width)
        }
        _ => data_type.clone(),
    }
}

pub(crate) fn sink_layer(schema: &Schema, options: &WriteOptions) -> SchemaEvolutionLayer {
    let remaps = schema
        .fields()
        .iter()
        .filter_map(|source| {
            let target = Arc::new(output_field(source, options));
            if source == &target {
                return None;
            }
            let source = source.clone();
            let output = target.clone();
            Some(FieldRemap::new(
                source.name().clone(),
                target,
                move |array| {
                    transform(array, &source, &output, None).map_err(|error| {
                        open_arrow_sink_core::Error::InvalidInput(error.to_string())
                    })
                },
            ))
        })
        .collect();
    SchemaEvolutionLayer::new(remaps)
}

fn output_field(field: &Field, options: &WriteOptions) -> Field {
    let mut metadata = field.metadata().clone();
    metadata.remove(SOURCE);
    if is_media(field) {
        let mut layout = BlobFieldOptions::default();
        if let Some(blob) = &options.blob_handle {
            layout.inline_size_threshold = blob.inline_size_threshold;
            layout.dedicated_size_threshold = blob.dedicated_size_threshold;
        }
        let blob = blob_field_with_options(field.name(), true, layout);
        metadata.extend(blob.metadata().clone());
        return blob.with_metadata(metadata);
    }
    let mut data_type = map_children(field.data_type(), &|child, _| output_field(child, options));
    if metadata.get(POLICY).map(String::as_str) == Some("text") {
        data_type = DataType::LargeBinary;
        metadata
            .entry("lance-encoding:compression".into())
            .or_insert_with(|| "lz4".into());
    }
    field
        .clone()
        .with_data_type(data_type)
        .with_metadata(metadata)
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
    if is_media(source) {
        return media(array, source, target, visible);
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

fn media(
    array: ArrayRef,
    source: &Field,
    target: &Field,
    visible: Option<NullBuffer>,
) -> Result<ArrayRef> {
    let source_url = validation::required(
        source.metadata().get(SOURCE),
        "ATIF media is missing its source URI",
    )?;
    let base = Url::parse(source_url)?;
    let paths = arrow_cast::cast(&array, &DataType::Utf8)?;
    let paths = paths.as_string::<i32>();
    let mut data = LargeBinaryBuilder::new();
    let mut uris = StringBuilder::new();
    for row in 0..paths.len() {
        if visible.as_ref().is_some_and(|nulls| nulls.is_null(row)) {
            data.append_null();
            uris.append_null();
            continue;
        }
        let path = paths.value(row);
        validation::validate_atif_media_reference(path)?;
        if path
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
        {
            let (header, payload) =
                validation::required(path.split_once(','), "invalid ATIF data URI")?;
            let payload = percent_encoding::percent_decode_str(payload).collect::<Vec<_>>();
            let bytes = if header.to_ascii_lowercase().ends_with(";base64") {
                STANDARD.decode(payload)?
            } else {
                payload
            };
            data.append_value(bytes);
            uris.append_null();
        } else {
            let uri = base.join(path)?;
            validation::validate_atif_media_uri(&base, &uri)?;
            data.append_null();
            uris.append_value(uri.as_str());
        }
    }
    let DataType::Struct(fields) = target.data_type() else {
        unreachable!("ATIF media target is a Blob struct")
    };
    Ok(Arc::new(StructArray::try_new(
        fields.clone(),
        vec![Arc::new(data.finish()), Arc::new(uris.finish())],
        visible,
    )?))
}
