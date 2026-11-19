//! Backend-specific Arrow C ABI opening.
use super::super::DuckportSink;
use crate::ffi::sink::{DuckportWriteOptions, open_sink};
use crate::{utils::string, validation};
use arrow_schema::ffi::FFI_ArrowSchema;
use open_arrow_sink_core::WriteOptions;
use std::ffi::c_char;

/// # Safety
/// Strings are borrowed NUL-terminated UTF-8. Schema/storage are live and borrowed.
/// Write options are live and borrowed. Output points to writable sink-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_iceberg_open(
    identifier: *const c_char,
    schema: *const FFI_ArrowSchema,
    mode: *const c_char,
    column_count: usize,
    catalog_uri: *const c_char,
    catalog_security: *const crate::security::DuckportSecurityOptions,
    warehouse: *const c_char,
    storage_root: *const c_char,
    options: *const DuckportWriteOptions,
    storage: *const crate::options::DuckportStorageOptions,
    output: *mut *mut DuckportSink,
) -> *mut c_char {
    unsafe {
        let open = |schema, write_options: WriteOptions, ingestion| async move {
            if validation::required(options.as_ref(), "null write options")?.blob_policy()?
                != open_arrow_sink_core::DownloadFailurePolicy::Fail
            {
                return Err("BLOB_DOWNLOAD_FAILURE_POLICY is supported only for Lance COPY".into());
            }
            let storage = validation::required(storage.as_ref(), "null Iceberg storage options")?;
            let mode = string(mode)?;
            let catalog_uri = string(catalog_uri)?;
            let storage_root = string(storage_root)?;
            validation::validate_iceberg_write_options(&mode, &catalog_uri, &storage_root)?;
            let write_options = write_options.with_mode(crate::utils::sink_mode(&mode)?);
            let catalog_security =
                validation::required(catalog_security.as_ref(), "null catalog security options")?
                    .read()?;
            super::open(
                &string(identifier)?,
                schema,
                &catalog_uri,
                &catalog_security,
                &string(warehouse)?,
                &storage_root,
                &storage.read()?,
                write_options,
                ingestion,
            )
            .await
        };
        open_sink(schema, column_count, options, output, open)
    }
}
