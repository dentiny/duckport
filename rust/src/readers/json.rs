use crate::source::open_file_source;
use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions},
    storage,
};
use open_arrow_access::{OuterCompression, ReadRequest, file_format::json::JsonFormat};
use std::ffi::c_char;

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_json_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let format = JsonFormat::infer();
        Ok(open_file_source(
            operator,
            ReadRequest::new(path).with_options(scan_options),
            format,
            OuterCompression::Auto,
            &storage_options.metrics,
        )
        .await?)
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}

#[cfg(test)]
mod tests {
    use super::duckport_json_open;
    use crate::metrics::DuckportReadMetrics;
    use crate::{error::ffi_result, ffi::source::*, options::*};
    use arrow_array::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
    use std::{ffi::CString, io::Write, ptr};

    #[test]
    fn file_scan_reports_live_progress_rows_and_encoded_io() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        for i in 0..20_000 {
            writeln!(file, "{{\"id\":{i},\"text\":\"{}\"}}", "x".repeat(128)).unwrap();
        }
        let path = CString::new(file.path().to_str().unwrap()).unwrap();
        let mut source = ptr::null_mut();
        unsafe {
            ffi_result(duckport_json_open(
                path.as_ptr(),
                &DuckportStorageOptions::default(),
                &DuckportScanOptions { batch_size: 8 },
                &mut source,
            ))
            .unwrap();
            let mut stream = FFI_ArrowArrayStream::empty();
            // COUNT(*)'s zero-column projection must still report all rows.
            ffi_result(duckport_source_get_stream(
                source,
                ptr::null(),
                0,
                &mut stream,
            ))
            .unwrap();
            let mut reader = ArrowArrayStreamReader::try_new(stream).unwrap();
            let first = reader.next().unwrap().unwrap();
            assert_eq!(first.num_columns(), 0);
            let mut live = DuckportReadMetrics::default();
            ffi_result(duckport_source_get_metrics(source, &mut live)).unwrap();
            assert!(live.total_bytes_known);
            assert_eq!(live.total_bytes, file.as_file().metadata().unwrap().len());
            assert!(live.progress_bytes > 0 && live.progress_bytes < live.total_bytes);
            assert!(!live.finished);
            let mut rows = first.num_rows();
            for batch in reader.by_ref() {
                rows += batch.unwrap().num_rows();
            }
            let mut final_metrics = DuckportReadMetrics::default();
            ffi_result(duckport_source_get_metrics(source, &mut final_metrics)).unwrap();
            assert!(final_metrics.finished);
            assert_eq!(rows, 20_000);
            assert_eq!(final_metrics.rows_read, rows as u64);
            assert!(final_metrics.bytes_read >= final_metrics.total_bytes);
            drop(reader);
            // A second scan gets its own progress window, while profiler counters
            // remain cumulative so the C++ bridge can report deltas.
            let mut stream = FFI_ArrowArrayStream::empty();
            ffi_result(duckport_source_get_stream(
                source,
                ptr::null(),
                0,
                &mut stream,
            ))
            .unwrap();
            let mut reader = ArrowArrayStreamReader::try_new(stream).unwrap();
            assert!(reader.next().unwrap().is_ok());
            let mut second = DuckportReadMetrics::default();
            ffi_result(duckport_source_get_metrics(source, &mut second)).unwrap();
            assert!(!second.finished);
            assert!(second.progress_bytes < second.total_bytes);
            drop(reader);
            ffi_result(duckport_source_get_metrics(source, &mut second)).unwrap();
            assert!(!second.finished); // LIMIT/cancellation is not EOF.
            duckport_source_destroy(source);
        }
    }
}
