use super::common::RUNTIME;
use crate::{
    error::{call, cleanup},
    sink::DuckportSink,
    utils,
    utils::string,
    validation,
};
use arrow_array::{
    RecordBatch, StructArray,
    ffi::{FFI_ArrowArray, from_ffi},
};
use arrow_schema::{Schema, ffi::FFI_ArrowSchema};
use std::{ffi::c_char, ptr, sync::Arc};

/// # Safety
/// Strings are NUL-terminated UTF-8; schema and storage are live and borrowed.
/// Columns points to column_count readable string pointers (or is null for zero columns).
/// Output points to writable sink-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_open(
    format: *const c_char,
    path: *const c_char,
    schema: *const FFI_ArrowSchema,
    mode: *const c_char,
    columns: *const *const c_char,
    column_count: usize,
    storage: *const crate::options::DuckportStorageOptions,
    output: *mut *mut DuckportSink,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null sink output")?;
        *output = ptr::null_mut();
        let schema = validation::required(schema.as_ref(), "null sink schema")?;
        let storage = validation::required(storage.as_ref(), "null sink options")?;
        let columns = utils::ffi_slice(columns, column_count, "null blob columns")?;
        let columns = columns
            .iter()
            .map(|column| string(*column))
            .collect::<crate::error::Result<std::collections::HashSet<_>>>()?;
        let sink = RUNTIME.block_on(DuckportSink::new(
            utils::sink_format(&string(format)?)?,
            &string(path)?,
            Arc::new(Schema::try_from(schema)?),
            &string(mode)?,
            columns,
            &storage.read()?,
        ))?;
        *output = Box::into_raw(Box::new(sink));
        Ok(())
    })
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
/// Sink is a live pointer returned by duckport_sink_open.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_commit(sink: *mut DuckportSink) -> *mut c_char {
    call(|| unsafe { RUNTIME.block_on(validation::required(sink.as_mut(), "null sink")?.commit()) })
}
/// # Safety
/// Sink is null or a live pointer returned by duckport_sink_open, freed once.
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
