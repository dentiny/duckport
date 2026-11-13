use crate::ffi::{
    DuckportSource, duckport_parquet_open, duckport_source_destroy, duckport_source_get_stream,
};
use arrow_array::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use std::{ffi::CString, mem::MaybeUninit, ptr};

pub(super) struct Fixture {
    source: *mut DuckportSource,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let path = CString::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../test/data/people.parquet")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let config = CString::new("{}").unwrap();
        let mut source = ptr::null_mut();
        let error =
            unsafe { duckport_parquet_open(path.as_ptr(), config.as_ptr(), 2, &mut source) };
        assert!(error.is_null());
        assert!(!source.is_null());
        Self { source }
    }

    pub(super) fn stream(&self) -> ArrowArrayStreamReader {
        let mut output = MaybeUninit::<FFI_ArrowArrayStream>::uninit();
        let error = unsafe { duckport_source_get_stream(self.source, output.as_mut_ptr()) };
        assert!(error.is_null());
        ArrowArrayStreamReader::try_new(unsafe { output.assume_init() }).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe { duckport_source_destroy(self.source) };
    }
}
