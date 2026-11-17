//! Shared Arrow C ABI write, commit and destruction.
use super::common::RUNTIME;
use crate::{
    error::{call, cleanup},
    sink::DuckportSink,
    validation,
};
use arrow_array::{
    RecordBatch, StructArray,
    ffi::{FFI_ArrowArray, from_ffi},
};
use arrow_schema::ffi::FFI_ArrowSchema;
use std::ffi::c_char;

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
/// Sink is a live pointer returned by duckport_lance_open.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_commit(sink: *mut DuckportSink) -> *mut c_char {
    call(|| unsafe { RUNTIME.block_on(validation::required(sink.as_mut(), "null sink")?.commit()) })
}
/// # Safety
/// Sink is null or a live pointer returned by duckport_lance_open, freed once.
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
