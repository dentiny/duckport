use crate::source::files::open as open_file_source;
use open_arrow_access::{OuterCompression, file_format::json::JsonFormat};

crate::ffi::reader::export_reader!(duckport_json_open;
    |locator, storage_options, scan_options| async move {
        let format = JsonFormat::infer();
        open_file_source(
            &locator,
            &storage_options,
            scan_options,
            format,
            OuterCompression::Auto,
        )
        .await
    }
);

#[cfg(test)]
mod tests {
    use super::duckport_json_open;
    use crate::{ffi::source::*, metrics::DuckportReadMetrics, options::*};
    use arrow_array::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
    use std::{ffi::CString, io::Write, ptr};

    #[test]
    fn file_scan_reports_live_progress_rows_and_encoded_io() {
        const ROWS: usize = 20_000;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let row = format!("{{\"id\":1,\"text\":\"{}\"}}\n", "x".repeat(128));
        file.write_all(row.repeat(ROWS).as_bytes()).unwrap();
        let path = CString::new(file.path().to_str().unwrap()).unwrap();
        let mut source = ptr::null_mut();
        let mut stream = FFI_ArrowArrayStream::empty();
        let mut metrics = DuckportReadMetrics::default();
        unsafe {
            assert!(
                duckport_json_open(
                    path.as_ptr(),
                    &DuckportStorageOptions::default(),
                    &DuckportScanOptions {
                        batch_size: 8,
                        files: ptr::null(),
                        file_count: 0
                    },
                    &mut source,
                )
                .is_null()
            );
            // COUNT(*) selects no columns, but must still count rows and track progress.
            assert!(duckport_source_get_stream(source, ptr::null(), 0, &mut stream).is_null());
            let mut reader = ArrowArrayStreamReader::try_new(stream).unwrap();
            let first = reader.next().unwrap().unwrap();
            assert_eq!(first.num_columns(), 0);
            assert!(duckport_source_get_metrics(source, &mut metrics).is_null());
            assert!(metrics.total_bytes_known);
            assert_eq!(
                metrics.total_bytes,
                file.as_file().metadata().unwrap().len()
            );
            assert!(metrics.progress_bytes > 0 && metrics.progress_bytes < metrics.total_bytes);
            assert!(!metrics.finished);

            let rows = reader.fold(first.num_rows(), |rows, batch| {
                rows + batch.unwrap().num_rows()
            });
            assert!(duckport_source_get_metrics(source, &mut metrics).is_null());
            assert!(metrics.finished);
            assert_eq!(rows, ROWS);
            assert_eq!(metrics.rows_read, ROWS as u64);
            assert!(metrics.bytes_read >= metrics.total_bytes);
            duckport_source_destroy(source);
        }
    }
}
