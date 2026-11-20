use std::{
    ffi::{CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
};

pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) const BHL_KEYS_NULL: &str = "null BHL Open Data key list";

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

/// Keep source URLs and credentials out of errors exposed by the HTTP transport.
pub(crate) fn http_source_error(message: &'static str) -> opendal::Error {
    opendal::Error::new(opendal::ErrorKind::Unexpected, message)
}

pub(crate) fn unknown_selected_file() -> open_arrow_access::Error {
    open_arrow_access::Error::new(
        open_arrow_access::ErrorKind::InvalidInput,
        "select_file",
        "unknown selected file",
    )
}

pub(crate) fn unsupported_semantic_scholar_dataset() -> open_arrow_access::Error {
    open_arrow_access::Error::new(
        open_arrow_access::ErrorKind::InvalidInput,
        "semantic_scholar",
        "Semantic Scholar only supports s2orc_v2",
    )
}
