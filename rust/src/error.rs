use arrow_array::RecordBatch;
use arrow_schema::ArrowError;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) fn read_batch(
    operation: impl FnOnce() -> open_arrow_access::Result<Option<RecordBatch>>,
) -> std::result::Result<Option<RecordBatch>, ArrowError> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(batch)) => Ok(batch),
        Ok(Err(error)) => Err(ArrowError::ExternalError(Box::new(error))),
        Err(_) => Err(ArrowError::ComputeError("source reader panicked".into())),
    }
}

use std::{
    ffi::{CString, c_char},
    ptr,
};

pub(crate) fn call(operation: impl FnOnce() -> Result<()>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => ptr::null_mut(),
        result => {
            let message = match result {
                Ok(Err(error)) => error.to_string(),
                _ => "source operation panicked".into(),
            };
            CString::new(message.replace('\0', " ")).unwrap().into_raw()
        }
    }
}

pub(crate) fn cleanup(operation: impl FnOnce()) {
    let _ = catch_unwind(AssertUnwindSafe(operation));
}

/// # Safety
/// Error is null or a pointer returned by these FFI functions, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_error_free(error: *mut c_char) {
    if !error.is_null() {
        unsafe {
            drop(CString::from_raw(error));
        }
    }
}
