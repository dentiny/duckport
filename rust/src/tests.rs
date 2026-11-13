mod fixture;

use crate::{
    error::duckport_source_error_free,
    ffi::{duckport_parquet_open, duckport_source_get_stream},
};
use fixture::Fixture;
use std::{ffi::CString, ptr};

#[test]
fn batches_and_rescans_preserve_rows_and_nulls() {
    let source = Fixture::new();
    for _ in 0..2 {
        let batches = source
            .stream()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches.iter().map(|b| b.num_rows()).sum::<usize>(), 3);
        assert_eq!(batches[0].column(1).null_count(), 1);
    }
}

#[test]
fn reader_outlives_factory_and_can_stop_early() {
    let source = Fixture::new();
    let mut reader = source.stream();
    drop(source);
    assert_eq!(reader.next().unwrap().unwrap().num_rows(), 2);
    drop(reader);
}

#[test]
fn concurrent_scans_share_runtime() {
    let workers: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(batches_and_rescans_preserve_rows_and_nulls))
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

#[test]
fn ffi_failures_return_owned_errors() {
    let missing = CString::new("missing.parquet").unwrap();
    let config = CString::new("{}").unwrap();
    let mut source = ptr::null_mut();
    unsafe {
        let error = duckport_parquet_open(missing.as_ptr(), config.as_ptr(), 2, &mut source);
        assert!(!error.is_null());
        assert!(source.is_null());
        duckport_source_error_free(error);
        let error = duckport_source_get_stream(ptr::null(), ptr::null_mut());
        assert!(!error.is_null());
        duckport_source_error_free(error);
    }
}
