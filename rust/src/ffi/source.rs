use super::common::RUNTIME;
use crate::{
    error::{Result, call, cleanup},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
    storage,
    utils::string,
    validation,
};
use arrow_array::ffi_stream::FFI_ArrowArrayStream;
use arrow_schema::ffi::FFI_ArrowSchema;
use open_arrow_access::{
    BlockingRecordBatchReader, OuterCompression, ReadOptions, ReadRequest,
    file_format::parquet::ParquetFormat, open_file_source,
};
use std::{ffi::c_char, ptr, sync::Arc};

pub struct DuckportSource {
    source: Option<AsyncSource>,
}

impl Drop for DuckportSource {
    fn drop(&mut self) {
        let _guard = RUNTIME.enter();
        self.source.take();
    }
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_parquet_source_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        Ok(open_file_source(
            operator,
            ReadRequest::new(path).with_options(scan_options),
            ParquetFormat,
            OuterCompression::None,
        )
        .await?
        .into())
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}

/// # Safety
/// Source is live; output points to writable, uninitialized ArrowSchema storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_get_schema(
    source: *const DuckportSource,
    output: *mut FFI_ArrowSchema,
) -> *mut c_char {
    call(|| unsafe {
        let source = validation::required(source.as_ref(), "null schema argument")?;
        validation::non_null(output, "null schema argument")?;
        ptr::write(
            output,
            FFI_ArrowSchema::try_from(source.source.as_ref().unwrap().schema().as_ref())?,
        );
        Ok(())
    })
}
/// # Safety
/// Source is live; output points to writable, uninitialized ArrowArrayStream storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_get_stream(
    source: *const DuckportSource,
    output: *mut FFI_ArrowArrayStream,
) -> *mut c_char {
    call(|| unsafe {
        let handle = validation::required(source.as_ref(), "null stream argument")?;
        validation::non_null(output, "null stream argument")?;
        let source = handle.source.as_ref().unwrap();
        let reader = BlockingRecordBatchReader::new(
            RUNTIME.block_on(source.reader())?,
            Arc::clone(&RUNTIME),
        );
        ptr::write(output, FFI_ArrowArrayStream::new(Box::new(reader)));
        Ok(())
    })
}
/// # Safety
/// Source is null or a live source pointer returned by an opener, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_destroy(source: *mut DuckportSource) {
    cleanup(|| {
        if !source.is_null() {
            unsafe {
                drop(Box::from_raw(source));
            }
        }
    });
}

pub(crate) unsafe fn open_reader<F, Fut>(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
    open: F,
) -> *mut c_char
where
    F: FnOnce(String, StorageConfig, ReadOptions) -> Fut,
    Fut: std::future::Future<Output = Result<AsyncSource>>,
{
    call(|| unsafe {
        validation::non_null(output, "null source output")?;
        *output = ptr::null_mut();
        let locator = string(locator)?;
        let storage = storage_options.read()?;
        let scan = scan_options.read()?;
        let source = RUNTIME.block_on(open(locator, storage, scan))?;
        *output = Box::into_raw(Box::new(DuckportSource {
            source: Some(source),
        }));
        Ok(())
    })
}
