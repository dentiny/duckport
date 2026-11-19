//! Observe the production JSON/Arrow FFI path for SQLLogicTest assertions.
use crate::{
    error::Result,
    ffi::source::{duckport_source_get_metrics, duckport_source_get_stream},
    metrics::DuckportReadMetrics,
    options::{DuckportScanOptions, DuckportStorageOptions},
    readers::duckport_json_open,
    validation,
};
use arrow_array::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use std::{
    ffi::{CString, c_char},
    ptr,
};

// Source FFI errors own CStrings, just like the fixture's diagnostic outputs.
unsafe fn check_error(error: *mut c_char) -> Result<()> {
    if error.is_null() {
        Ok(())
    } else {
        Err(unsafe { CString::from_raw(error) }
            .to_string_lossy()
            .into_owned()
            .into())
    }
}

fn snapshot(metrics: DuckportReadMetrics) -> String {
    format!(
        "{{\"bytes_read\":{},\"rows_read\":{},\"total_bytes\":{},\"progress_bytes\":{},\"total_bytes_known\":{},\"finished\":{}}}",
        metrics.bytes_read,
        metrics.rows_read,
        metrics.total_bytes,
        metrics.progress_bytes,
        metrics.total_bytes_known,
        metrics.finished,
    )
}

unsafe fn scan(locator: *const c_char) -> Result<String> {
    unsafe {
        let mut raw = ptr::null_mut();
        check_error(duckport_json_open(
            locator,
            &DuckportStorageOptions::default(),
            &DuckportScanOptions {
                read_concurrency: 1,
                batch_size: 8,
                files: ptr::null(),
                file_count: 0,
            },
            &mut raw,
        ))?;
        validation::non_null(raw, "missing fixture source")?;
        // Own the source immediately, including on errors while reading its stream.
        let source = Box::from_raw(raw);
        let mut stream = FFI_ArrowArrayStream::empty();
        check_error(duckport_source_get_stream(
            source.as_ref(),
            ptr::null(),
            0,
            &mut stream,
        ))?;
        let mut reader = ArrowArrayStreamReader::try_new(stream)?;
        let first = reader
            .next()
            .transpose()?
            .ok_or("JSON metrics fixture requires rows")?;
        let columns = first.num_columns();
        let mut rows = first.num_rows();
        let mut initial = DuckportReadMetrics::default();
        check_error(duckport_source_get_metrics(source.as_ref(), &mut initial))?;
        for batch in reader {
            rows += batch?.num_rows();
        }
        let mut complete = DuckportReadMetrics::default();
        check_error(duckport_source_get_metrics(source.as_ref(), &mut complete))?;
        Ok(format!(
            "{{\"initial\":{},\"complete\":{},\"rows\":{rows},\"columns\":{columns}}}",
            snapshot(initial),
            snapshot(complete),
        ))
    }
}

/// # Safety
/// Locator is a borrowed NUL-terminated string. Output follows the fixture
/// diagnostic ownership contract and is freed with duckport_source_error_free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_fixture_json_metrics(
    locator: *const c_char,
    output: *mut *mut c_char,
) -> *mut c_char {
    unsafe { super::string_output(output, || scan(locator)) }
}
