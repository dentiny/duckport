//! Lance Arrow C ABI opener.
use super::super::DuckportSink;
use crate::{
    error::{Result, call},
    ffi::common::RUNTIME,
    utils,
    utils::string,
    validation,
};
use arrow_schema::{Schema, ffi::FFI_ArrowSchema};
use open_arrow_sink_core::{BlobHandleConfig, WriteOptions};
use std::{ffi::c_char, ptr, sync::Arc};

/// Byte sizes passed across the C ABI. -1 leaves a backend default unchanged.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DuckportLanceWriteOptions {
    pub blob_inline_size_threshold: i64,
    pub blob_dedicated_size_threshold: i64,
    pub target_file_size: i64,
}

impl Default for DuckportLanceWriteOptions {
    fn default() -> Self {
        Self {
            blob_inline_size_threshold: -1,
            blob_dedicated_size_threshold: -1,
            target_file_size: -1,
        }
    }
}

impl DuckportLanceWriteOptions {
    pub(super) fn read(&self) -> Result<WriteOptions> {
        fn size(name: &str, value: i64, allow_zero: bool) -> Result<Option<usize>> {
            if value == -1 {
                return Ok(None);
            }
            if value < 0 || (!allow_zero && value == 0) {
                return Err(format!("{name} has an invalid byte size").into());
            }
            Ok(Some(
                usize::try_from(value).map_err(|_| format!("{name} exceeds platform size"))?,
            ))
        }
        let inline = size(
            "BLOB_INLINE_SIZE_THRESHOLD",
            self.blob_inline_size_threshold,
            true,
        )?;
        let dedicated = size(
            "BLOB_DEDICATED_SIZE_THRESHOLD",
            self.blob_dedicated_size_threshold,
            false,
        )?;
        let mut options = WriteOptions::default();
        options.target_file_size = size("TARGET_FILE_SIZE", self.target_file_size, false)?;
        if inline.is_some() || dedicated.is_some() {
            options.blob_handle = Some(BlobHandleConfig {
                inline_size_threshold: inline,
                dedicated_size_threshold: dedicated.and_then(std::num::NonZeroUsize::new),
                ..Default::default()
            });
        }
        Ok(options)
    }
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
    options: *const DuckportLanceWriteOptions,
    storage: *const crate::options::DuckportStorageOptions,
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
        let sink = RUNTIME.block_on(super::open(
            &string(path)?,
            Arc::new(Schema::try_from(schema)?),
            &string(mode)?,
            columns,
            &storage.read()?,
            options.read()?,
        ))?;
        *output = Box::into_raw(Box::new(sink));
        Ok(())
    })
}
