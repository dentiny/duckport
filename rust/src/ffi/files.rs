//! C ABI and synchronous runtime boundary for file discovery.
use super::common::RUNTIME;
use crate::{
    error::{call, cleanup},
    options::DuckportStorageOptions,
    utils, validation,
};
use std::{
    ffi::{CString, c_char},
    ptr,
};

pub struct DuckportFileList(Vec<CString>);

/// # Safety
/// Locator is borrowed and NUL-terminated; output is writable. The returned
/// one-element list owns the credential lookup URI until destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_discovery_root(
    locator: *const c_char,
    output: *mut *mut DuckportFileList,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null discovery root output")?;
        *output = ptr::null_mut();
        let root = crate::files::credential_scope(&utils::string(locator)?)?;
        *output = Box::into_raw(Box::new(DuckportFileList(vec![CString::new(root)?])));
        Ok(())
    })
}

/// # Safety
/// Strings/options are borrowed for this call; output is writable. Free its
/// returned handle once with duckport_file_list_destroy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_resolve_files(
    locator: *const c_char,
    format: *const c_char,
    storage: &DuckportStorageOptions,
    output: *mut *mut DuckportFileList,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null file selection output")?;
        *output = ptr::null_mut();
        let files = RUNTIME.block_on(crate::files::resolve(
            &utils::string(locator)?,
            &utils::string(format)?,
            &storage.read()?,
        ))?;
        let files = files
            .into_iter()
            .map(CString::new)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        *output = Box::into_raw(Box::new(DuckportFileList(files)));
        Ok(())
    })
}
/// # Safety
/// The handle is live and borrowed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_list_count(files: &DuckportFileList) -> usize {
    files.0.len()
}
/// # Safety
/// The handle is live and index is less than duckport_file_list_count. The
/// returned string is borrowed until the handle is destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_list_get(
    files: &DuckportFileList,
    index: usize,
) -> *const c_char {
    files.0[index].as_ptr()
}
/// # Safety
/// The handle is null or live, and freed only once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_list_destroy(files: *mut DuckportFileList) {
    cleanup(|| {
        if !files.is_null() {
            drop(unsafe { Box::from_raw(files) });
        }
    });
}
