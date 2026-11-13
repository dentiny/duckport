use crate::{
    error::duckport_source_error_free,
    ffi::{DuckportSource, duckport_parquet_open, duckport_source_get_stream},
};
use arrow_array::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use std::{ffi::CString, mem::MaybeUninit, ptr};

fn fixture() -> Box<DuckportSource> {
    let path = CString::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test/data/people.parquet")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let config = CString::new("{}").unwrap();
    let mut source = ptr::null_mut();
    let error = unsafe { duckport_parquet_open(path.as_ptr(), config.as_ptr(), 2, &mut source) };
    assert!(error.is_null());
    unsafe { Box::from_raw(source) }
}

fn stream(source: &DuckportSource) -> ArrowArrayStreamReader {
    let mut output = MaybeUninit::<FFI_ArrowArrayStream>::uninit();
    let error = unsafe { duckport_source_get_stream(source, output.as_mut_ptr()) };
    assert!(error.is_null());
    ArrowArrayStreamReader::try_new(unsafe { output.assume_init() }).unwrap()
}

#[test]
fn batches_and_rescans_preserve_rows_and_nulls() {
    let source = fixture();
    for _ in 0..2 {
        let batches = stream(&source)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches.iter().map(|b| b.num_rows()).sum::<usize>(), 3);
        assert_eq!(batches[0].column(1).null_count(), 1);
    }
}

#[test]
fn reader_outlives_factory_and_can_stop_early() {
    let source = fixture();
    let mut reader = stream(&source);
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
