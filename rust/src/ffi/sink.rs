//! Shared Arrow C ABI write options, write, commit and destruction.
use super::common::RUNTIME;
use crate::{
    error::{Result, call, cleanup},
    sink::DuckportSink,
    utils::{optional_string, string},
    validation,
};
use arrow_array::{
    RecordBatch, StructArray,
    ffi::{FFI_ArrowArray, from_ffi},
};
use arrow_schema::ffi::FFI_ArrowSchema;
use open_arrow_sink_core::{BlobHandleConfig, WriteOptions};
use std::ffi::c_char;

/// # Safety
/// Name is a borrowed NUL-terminated UTF-8 write-option name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_validate_byte_size(
    name: *const c_char,
    value: i64,
) -> *mut c_char {
    call(|| unsafe {
        validation::byte_size(&string(name)?, value)?;
        Ok(())
    })
}

/// Shared C ABI write options. -1 byte sizes and a null version preserve backend defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DuckportWriteOptions {
    pub blob_inline_size_threshold: i64,
    pub blob_dedicated_size_threshold: i64,
    pub target_file_size: i64,
    pub storage_version: *const c_char,
}

impl Default for DuckportWriteOptions {
    fn default() -> Self {
        Self {
            blob_inline_size_threshold: -1,
            blob_dedicated_size_threshold: -1,
            target_file_size: -1,
            storage_version: std::ptr::null(),
        }
    }
}

impl DuckportWriteOptions {
    // Selecting URL columns requests blob support even without explicit layout overrides.
    /// # Safety
    /// storage_version is null or a borrowed NUL-terminated UTF-8 string.
    pub(crate) unsafe fn convert(&self, blob_column_count: usize) -> Result<WriteOptions> {
        let inline = validation::optional_byte_size(
            "BLOB_INLINE_SIZE_THRESHOLD",
            self.blob_inline_size_threshold,
        )?;
        let dedicated = validation::optional_byte_size(
            "BLOB_DEDICATED_SIZE_THRESHOLD",
            self.blob_dedicated_size_threshold,
        )?;
        let mut options = WriteOptions::default();
        options.storage_version = unsafe { optional_string(self.storage_version) }?;
        options.target_file_size =
            validation::optional_byte_size("TARGET_FILE_SIZE", self.target_file_size)?;
        if blob_column_count != 0 || inline.is_some() || dedicated.is_some() {
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
/// Sink is live. Array is an owned Arrow struct array; it is consumed even on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_write(
    sink: *mut DuckportSink,
    array: *mut FFI_ArrowArray,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(array, "null sink array")?;
        let array = FFI_ArrowArray::from_raw(array);
        let sink = validation::required(sink.as_mut(), "null sink")?;
        let schema = FFI_ArrowSchema::try_from(sink.schema.as_ref())?;
        let data = from_ffi(array, &schema)?;
        let batch = RecordBatch::from(StructArray::from(data)).with_schema(sink.schema.clone())?;
        RUNTIME.block_on(sink.write(batch))
    })
}
/// # Safety
/// Sink is a live pointer returned by a sink open function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_commit(sink: *mut DuckportSink) -> *mut c_char {
    call(|| unsafe { RUNTIME.block_on(validation::required(sink.as_mut(), "null sink")?.commit()) })
}
/// # Safety
/// Sink is null or a live pointer returned by a sink open function, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_destroy(sink: *mut DuckportSink) {
    cleanup(|| {
        if !sink.is_null() {
            let _guard = RUNTIME.enter();
            unsafe {
                drop(Box::from_raw(sink));
            }
        }
    });
}
