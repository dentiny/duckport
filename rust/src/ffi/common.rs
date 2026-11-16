use crate::error::Result;
use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::{Arc, LazyLock},
};
use tokio::runtime::Runtime;

pub(crate) static RUNTIME: LazyLock<Arc<Runtime>> = LazyLock::new(|| {
    Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create Arrow runtime"),
    )
});

pub(crate) unsafe fn string(value: *const c_char) -> Result<String> {
    if value.is_null() {
        return Err("null string argument".into());
    }
    Ok(unsafe { CStr::from_ptr(value) }.to_str()?.to_owned())
}

pub(crate) unsafe fn optional_string(value: *const c_char) -> Result<Option<String>> {
    if value.is_null() {
        Ok(None)
    } else {
        unsafe { string(value) }.map(Some)
    }
}

pub(crate) fn call(operation: impl FnOnce() -> Result<()>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => ptr::null_mut(),
        result => {
            let message = match result {
                Ok(Err(error)) => error.to_string(),
                _ => "FFI operation panicked".into(),
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
