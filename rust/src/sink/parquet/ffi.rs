//! Parquet Arrow C ABI opener.
use super::super::DuckportSink;
use crate::{error::call, ffi::common::RUNTIME, utils::string, validation};
use arrow_schema::{Schema, ffi::FFI_ArrowSchema};
use std::{ffi::c_char, ptr, sync::Arc};

/// # Safety
/// Strings are NUL-terminated UTF-8; schema and storage are live and borrowed.
/// Output points to writable sink-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_parquet_open(
    path: *const c_char,
    schema: *const FFI_ArrowSchema,
    mode: *const c_char,
    storage: *const crate::options::DuckportStorageOptions,
    output: *mut *mut DuckportSink,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null sink output")?;
        *output = ptr::null_mut();
        let schema = validation::required(schema.as_ref(), "null sink schema")?;
        let storage = validation::required(storage.as_ref(), "null Parquet options")?;
        let sink = RUNTIME.block_on(super::open(
            &string(path)?,
            Arc::new(Schema::try_from(schema)?),
            &string(mode)?,
            &storage.read()?,
        ))?;
        *output = Box::into_raw(Box::new(sink));
        Ok(())
    })
}
