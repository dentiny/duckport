//! Lance Arrow C ABI opener.
use super::super::DuckportSink;
use crate::ffi::sink::{DuckportWriteOptions, open_sink};
use crate::{utils, utils::string, validation};
use arrow_schema::ffi::FFI_ArrowSchema;
use std::ffi::c_char;

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
    unsafe {
        let open = |schema, write_options, ingestion| async move {
            let storage = validation::required(storage.as_ref(), "null Lance options")?;
            let columns = utils::ffi_slice(columns, column_count, "null Lance columns")?;
            let columns = columns
                .iter()
                .map(|column| string(*column))
                .collect::<crate::error::Result<std::collections::HashSet<_>>>()?;
            let sources = utils::ffi_slice(sources, source_count, "null blob sources")?
                .iter()
                .map(|source| {
                    Ok(super::BlobSource {
                        document: string(source.document)?,
                        storage: source.storage.read()?,
                    })
                })
                .collect::<crate::error::Result<Vec<_>>>()?;
            super::open(
                &string(path)?,
                schema,
                &string(mode)?,
                columns,
                &storage.read()?,
                write_options,
                ingestion,
                crate::blob_auth::read_secrets(secrets, secret_count)?,
                sources,
            )
            .await
        };
        open_sink(schema, column_count, options, output, open)
    }
}
