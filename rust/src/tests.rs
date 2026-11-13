
use crate::{
    error::duckport_source_error_free,
    ffi::{duckport_parquet_open, duckport_source_get_stream},
    source::DuckportSource,
};
use std::{ffi::CString, ptr};
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
#[test]
fn batches_and_rescans_preserve_rows_and_nulls() {
    let source = fixture();
    for _ in 0..2 {
        let batches = source
            .reader()
            .unwrap()
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
    let mut reader = source.reader().unwrap();
    drop(source);
    assert_eq!(reader.next().unwrap().unwrap().num_rows(), 2);
    drop(reader);
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
