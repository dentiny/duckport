//! Parquet Arrow C ABI opener.
use super::super::DuckportSink;
use crate::ffi::sink::{DuckportWriteOptions, open_sink};
use crate::{utils::string, validation};
use arrow_schema::ffi::FFI_ArrowSchema;
use std::ffi::c_char;

/// # Safety
/// Strings are NUL-terminated UTF-8; schema and storage are live and borrowed.
/// Write options are live and borrowed. Output points to writable sink-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_parquet_open(
    path: *const c_char,
    schema: *const FFI_ArrowSchema,
    mode: *const c_char,
    column_count: usize,
    options: *const DuckportWriteOptions,
    storage: *const crate::options::DuckportStorageOptions,
    output: *mut *mut DuckportSink,
) -> *mut c_char {
    unsafe {
        let open = |schema, write_options, ingestion| async move {
            let storage = validation::required(storage.as_ref(), "null Parquet options")?;
            super::open(
                &string(path)?,
                schema,
                &string(mode)?,
                &storage.read()?,
                write_options,
                ingestion,
            )
            .await
        };
        open_sink(schema, column_count, options, output, open)
    }
}
