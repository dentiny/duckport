//! Read-only diagnostics for SQLLogicTest fixtures; excluded from production builds.
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
            for row in 0..values.len() {
                count(values.as_ref(), row, path, kinds)?;
            }
        }
        DataType::LargeList(_) => {
            let values = array.as_list::<i64>().value(row);
            for row in 0..values.len() {
                count(values.as_ref(), row, path, kinds)?;
            }
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

/// # Safety
/// Inputs are borrowed NUL-terminated strings; output is writable and initially
/// unowned. Success returns an owned CString, freed with duckport_source_error_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_fixture_blob_layout(
    locator: *const c_char,
    field: *const c_char,
    output: *mut *mut c_char,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null fixture output")?;
        *output = ptr::null_mut();
        let locator = utils::string(locator)?;
        let path = utils::local_path(&locator)?;
        let field = utils::string(field)?;
        let kinds = RUNTIME.block_on(async {
            let dataset = LanceDatasetSink::default()
                .dataset_builder(
                    url::Url::from_directory_path(path).map_err(|_| "invalid fixture path")?,
                )?
                .load()
                .await?;
            // The native scanner defaults to physical Blob descriptions, not bytes.
            let batch = dataset.scan().try_into_batch().await?;
            let root = StructArray::from(batch);
            let mut kinds = [0_u64; 6];
            let field = field.split('.').collect::<Vec<_>>();
            for row in 0..root.len() {
                count(&root, row, &field, &mut kinds)?;
            }
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(kinds)
        })?;
        let json = format!(
            "{{\"inline\":{},\"packed\":{},\"dedicated\":{},\"external\":{},\"managed\":{}}}",
            kinds[0],
            kinds[1],
            kinds[2],
            kinds[3],
            kinds[4] + kinds[5]
        );
        *output = CString::new(json)?.into_raw();
        Ok(())
    })
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
    call(|| unsafe {
        validation::non_null(output, "null fixture output")?;
        *output = ptr::null_mut();
        let path = utils::local_path(&utils::string(locator)?)?;
        let field = utils::string(field)?;
        let key = utils::string(key)?;
        let value = RUNTIME.block_on(async {
            let dataset = LanceDatasetSink::default()
                .dataset_builder(
                    url::Url::from_directory_path(path).map_err(|_| "invalid fixture path")?,
                )?
                .load()
                .await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
                dataset
                    .schema()
                    .field(&field)
                    .ok_or("unknown fixture field")?
                    .metadata
                    .get(&key)
                    .cloned()
                    .unwrap_or_default(),
            )
        })?;
        *output = CString::new(value)?.into_raw();
        Ok(())
    })
}

/// Inspect only the credential kind, without exposing any credential material.
/// # Safety
/// Options and output are live for the call, following the production C ABI contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_fixture_security(
    raw: *const crate::security::DuckportSecurityOptions,
    output: *mut *mut c_char,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null fixture output")?;
        *output = ptr::null_mut();
        let raw = validation::required(raw.as_ref(), "null fixture credentials")?;
        let kind = match raw.read()? {
            crate::security::SecurityConfig::None => "none",
            crate::security::SecurityConfig::Http(credentials) => {
                if !credentials
                    .headers()
                    .values()
                    .all(|value| value.is_sensitive())
                {
                    return Err("HTTP credential headers must be sensitive".into());
                }
                "http"
            }
            crate::security::SecurityConfig::S3(crate::security::S3Credentials::Anonymous) => {
                "s3-anonymous"
            }
            crate::security::SecurityConfig::S3(crate::security::S3Credentials::Static {
                ..
            }) => "s3-static",
        };
        *output = CString::new(kind)?.into_raw();
        Ok(())
    })
}
