//! Lance Arrow C ABI opener.
use super::super::DuckportSink;
use crate::ffi::sink::DuckportWriteOptions;
use crate::{error::call, ffi::common::RUNTIME, utils, utils::string, validation};
use arrow_schema::{Schema, ffi::FFI_ArrowSchema};
use std::{ffi::c_char, ptr, sync::Arc};

#[repr(C)]
pub struct DuckportBlobSource {
    pub document: *const c_char,
    pub storage: crate::options::DuckportStorageOptions,
}

/// # Safety
/// Strings are NUL-terminated UTF-8; schema and storage are live and borrowed.
/// Columns points to column_count readable string pointers (or is null for zero columns).
/// Write options and output are non-null. Output points to writable sink-pointer storage.
/// No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_lance_open(
    path: *const c_char,
    schema: *const FFI_ArrowSchema,
    mode: *const c_char,
    columns: *const *const c_char,
    column_count: usize,
    options: *const DuckportWriteOptions,
    secrets: *const crate::blob_auth::DuckportBlobSecret,
    secret_count: usize,
    storage: *const crate::options::DuckportStorageOptions,
    sources: *const DuckportBlobSource,
    source_count: usize,
    output: *mut *mut DuckportSink,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null sink output")?;
        *output = ptr::null_mut();
        let schema = validation::required(schema.as_ref(), "null sink schema")?;
        let storage = validation::required(storage.as_ref(), "null Lance options")?;
        let options = validation::required(options.as_ref(), "null Lance write options")?;
        let columns = utils::ffi_slice(columns, column_count, "null Lance columns")?;
        let columns = columns
            .iter()
            .map(|column| string(*column))
            .collect::<crate::error::Result<std::collections::HashSet<_>>>()?;
        let schema = Arc::new(Schema::try_from(schema)?);
        let write_options = options.convert(column_count, &schema)?;
        let sources = utils::ffi_slice(sources, source_count, "null blob sources")?
            .iter()
            .map(|source| {
                Ok(super::BlobSource {
                    document: string(source.document)?,
                    storage: source.storage.read()?,
                })
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        let sink = RUNTIME.block_on(super::open(
            &string(path)?,
            schema,
            &string(mode)?,
            columns,
            &storage.read()?,
            write_options,
            options.writer_count,
            crate::blob_auth::read_secrets(secrets, secret_count)?,
            sources,
        ))?;
        *output = Box::into_raw(Box::new(sink));
        Ok(())
    })
}
