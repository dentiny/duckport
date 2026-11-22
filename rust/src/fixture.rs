//! SQLLogicTest fixtures and diagnostics; excluded from production builds.
use crate::{
    error::{Result, call},
    ffi::common::RUNTIME,
    utils, validation,
};
use arrow_array::{Array, StructArray, cast::AsArray};
use arrow_schema::DataType;
use open_arrow_sink_lance::LanceDatasetSink;
use std::{
    ffi::{CString, c_char},
    ptr,
};

// Traverse only visible list/struct elements. Null ancestors must never contribute
// hidden descriptor values to SQL's physical-layout counts.
fn count(array: &dyn Array, row: usize, path: &[&str], kinds: &mut [u64; 6]) -> Result<()> {
    if array.is_null(row) {
        return Ok(());
    }
    match array.data_type() {
        DataType::List(_) => {
            let values = array.as_list::<i32>().value(row);
            count_rows(values.as_ref(), path, kinds)?;
        }
        DataType::LargeList(_) => {
            let values = array.as_list::<i64>().value(row);
            count_rows(values.as_ref(), path, kinds)?;
        }
        DataType::Struct(_) if !path.is_empty() => {
            let child = array
                .as_struct()
                .column_by_name(path[0])
                .ok_or("unknown fixture field path")?;
            count(child.as_ref(), row, &path[1..], kinds)?;
        }
        DataType::Struct(_) => {
            let descriptor = array.as_struct();
            let kind = descriptor
                .column_by_name("kind")
                .ok_or("fixture field is not a Blob descriptor")?
                .as_primitive::<arrow_array::types::UInt8Type>()
                .value(row) as usize;
            *kinds.get_mut(kind).ok_or("unknown Blob descriptor kind")? += 1;
        }
        _ => return Err("fixture path does not select a Blob field".into()),
    }
    Ok(())
}

fn count_rows(array: &dyn Array, path: &[&str], kinds: &mut [u64; 6]) -> Result<()> {
    for row in 0..array.len() {
        count(array, row, path, kinds)?;
    }
    Ok(())
}

fn dataset_url(locator: &str) -> Result<url::Url> {
    url::Url::from_directory_path(utils::local_path(locator)?)
        .map_err(|_| "invalid fixture path".into())
}

async fn blob_layout(locator: &str, field: &str) -> Result<String> {
    let dataset = LanceDatasetSink::default()
        .dataset_builder(dataset_url(locator)?)?
        .load()
        .await?;
    // The native scanner defaults to physical Blob descriptions, not bytes.
    let batch = dataset.scan().try_into_batch().await?;
    let root = StructArray::from(batch);
    let mut kinds = [0_u64; 6];
    let field = field.split('.').collect::<Vec<_>>();
    count_rows(&root, &field, &mut kinds)?;
    Ok(format!(
        "{{\"inline\":{},\"packed\":{},\"dedicated\":{},\"external\":{},\"managed\":{}}}",
        kinds[0],
        kinds[1],
        kinds[2],
        kinds[3],
        kinds[4] + kinds[5]
    ))
}

async fn field_metadata(locator: &str, field: &str, key: &str) -> Result<String> {
    let dataset = LanceDatasetSink::default()
        .dataset_builder(dataset_url(locator)?)?
        .load()
        .await?;
    Ok(dataset
        .schema()
        .field(field)
        .ok_or("unknown fixture field")?
        .metadata
        .get(key)
        .cloned()
        .unwrap_or_default())
}

/// # Safety
/// Output is writable and initially unowned. Success transfers an owned CString.
unsafe fn string_output(
    output: *mut *mut c_char,
    read: impl FnOnce() -> Result<String>,
) -> *mut c_char {
    call(|| {
        validation::non_null(output, "null fixture output")?;
        unsafe { *output = ptr::null_mut() };
        let value = CString::new(read()?)?;
        unsafe { *output = value.into_raw() };
        Ok(())
    })
}

/// # Safety
/// Inputs are borrowed NUL-terminated strings; output is writable and initially
/// unowned. Success returns an owned CString, freed with duckport_source_error_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_fixture_blob_layout(
    locator: *const c_char,
    field: *const c_char,
    output: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        string_output(output, || {
            let locator = utils::string(locator)?;
            let field = utils::string(field)?;
            RUNTIME.block_on(blob_layout(&locator, &field))
        })
    }
}

/// # Safety
/// Inputs and output follow duckport_fixture_blob_layout's ownership contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_fixture_field_metadata(
    locator: *const c_char,
    field: *const c_char,
    key: *const c_char,
    output: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        string_output(output, || {
            let locator = utils::string(locator)?;
            let field = utils::string(field)?;
            let key = utils::string(key)?;
            RUNTIME.block_on(field_metadata(&locator, &field, &key))
        })
    }
}

/// # Safety
/// Inputs and output follow duckport_fixture_blob_layout's ownership contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_fixture_bhl_metadata(
    root: *const c_char,
    database: *const c_char,
    output: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        string_output(output, || {
            let root = utils::local_path(&utils::string(root)?)?;
            let database = utils::string(database)?;
            open_arrow_access::storage::opendal::install_default();
            let storage = open_arrow_access::storage::OpenDalAccessor::new(
                crate::storage::create_local_directory_operator(&root)?,
            );
            RUNTIME.block_on(
                open_arrow_access::service::bhl::metadata::BhlMetadata::create(storage, &database),
            )?;
            Ok(database)
        })
    }
}
