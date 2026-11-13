use crate::{
    error::{Result, call, cleanup},
    source::DuckportSource,
    storage,
};
use arrow_array::ffi_stream::FFI_ArrowArrayStream;
use arrow_schema::ffi::FFI_ArrowSchema;
use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    num::NonZeroUsize,
    ptr,
};

unsafe fn string(value: *const c_char) -> Result<String> {
    if value.is_null() {
        return Err("null string argument".into());
    }
    Ok(unsafe { CStr::from_ptr(value) }.to_str()?.to_owned())
}

/// # Safety
/// Strings are NUL-terminated UTF-8; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_parquet_open(
    path: *const c_char,
    storage_options: *const c_char,
    batch_size: usize,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    call(|| unsafe {
        if output.is_null() {
            return Err("null source output".into());
        }
        *output = ptr::null_mut();
        let path = string(path)?;
        let config: HashMap<String, String> = serde_json::from_str(&string(storage_options)?)?;
        let batch_size = NonZeroUsize::new(batch_size)
            .filter(|s| s.get() <= 1048576)
            .ok_or("batch_size must be between 1 and 1048576")?;
        let (operator, path) = storage::operator(&path, config)?;
        let source = DuckportSource::new(operator, path, batch_size)?;
        *output = Box::into_raw(Box::new(source));
        Ok(())
    })
}

/// # Safety
/// Source is live; output points to writable, uninitialized ArrowSchema storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_get_schema(
    source: *const DuckportSource,
    output: *mut FFI_ArrowSchema,
) -> *mut c_char {
    call(|| unsafe {
        if source.is_null() || output.is_null() {
            return Err("null schema argument".into());
        }
        ptr::write(
            output,
            FFI_ArrowSchema::try_from((*source).schema().as_ref())?,
        );
        Ok(())
    })
}
/// # Safety
/// Source is live; output points to writable, uninitialized ArrowArrayStream storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_get_stream(
    source: *const DuckportSource,
    output: *mut FFI_ArrowArrayStream,
) -> *mut c_char {
    call(|| unsafe {
        if source.is_null() || output.is_null() {
            return Err("null stream argument".into());
        }
        ptr::write(
            output,
            FFI_ArrowArrayStream::new(Box::new((*source).reader()?)),
        );
        Ok(())
    })
}
/// # Safety
/// Source is null or a pointer returned by duckport_parquet_open, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_destroy(source: *mut DuckportSource) {
    cleanup(|| {
        if !source.is_null() {
            unsafe {
                drop(Box::from_raw(source));
            }
        }
    });
}
