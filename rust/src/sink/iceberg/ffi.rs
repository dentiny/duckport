//! Backend-specific Arrow C ABI opening.
use super::super::DuckportSink;
use crate::{error::call, ffi::common::RUNTIME, utils::string, validation};
use arrow_schema::{Schema, ffi::FFI_ArrowSchema};
use std::{ffi::c_char, ptr, sync::Arc};

/// # Safety
/// Strings are borrowed NUL-terminated UTF-8. Schema/storage are live and borrowed.
/// Output points to writable sink-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_iceberg_open(
    identifier: *const c_char,
    schema: *const FFI_ArrowSchema,
    mode: *const c_char,
    column_count: usize,
    catalog_uri: *const c_char,
    warehouse: *const c_char,
    storage_root: *const c_char,
    storage: *const crate::options::DuckportStorageOptions,
    output: *mut *mut DuckportSink,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null sink output")?;
        *output = ptr::null_mut();
        let schema = validation::required(schema.as_ref(), "null sink schema")?;
        let storage = validation::required(storage.as_ref(), "null Iceberg storage options")?;
        let mode = string(mode)?;
        let catalog_uri = string(catalog_uri)?;
        let storage_root = string(storage_root)?;
        validation::validate_iceberg_write_options(
            &mode,
            column_count,
            &catalog_uri,
            &storage_root,
        )?;
        let sink = RUNTIME.block_on(super::open(
            &string(identifier)?,
            Arc::new(Schema::try_from(schema)?),
            &catalog_uri,
            &string(warehouse)?,
            &storage_root,
            &storage.read()?,
        ))?;
        *output = Box::into_raw(Box::new(sink));
        Ok(())
    })
}
