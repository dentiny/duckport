use crate::{
    error::{Result, call, cleanup},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
    storage,
};
use arrow_array::{RecordBatch, RecordBatchReader, ffi_stream::FFI_ArrowArrayStream};
use arrow_schema::{ArrowError, SchemaRef, ffi::FFI_ArrowSchema};
use open_arrow_access::{
    ReadOptions, ReadRequest, RecordReader, file_format::parquet::ParquetAccessor,
    storage::OpenDalAccessor,
};
use std::{
    ffi::{CStr, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::LazyLock,
};
use tokio::runtime::Runtime;

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create source runtime")
});

pub struct DuckportSource {
    source: Option<AsyncSource>,
}

impl Drop for DuckportSource {
    fn drop(&mut self) {
        let _guard = RUNTIME.enter();
        self.source.take();
    }
}

struct BatchReader {
    reader: Option<Box<dyn RecordReader>>,
    schema: SchemaRef,
    done: bool,
}

impl Iterator for BatchReader {
    type Item = std::result::Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let batch = catch_unwind(AssertUnwindSafe(|| {
            RUNTIME.block_on(self.reader.as_mut().unwrap().next())
        }))
        .map_err(|_| ArrowError::ComputeError("source reader panicked".into()))
        .and_then(|result| result.map_err(|error| ArrowError::ExternalError(Box::new(error))));
        match batch {
            Ok(batch) => {
                self.done = batch.is_none();
                batch.map(Ok)
            }
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
}

impl RecordBatchReader for BatchReader {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

impl Drop for BatchReader {
    fn drop(&mut self) {
        let _guard = RUNTIME.enter();
        self.reader.take();
    }
}

unsafe fn string(value: *const c_char) -> Result<String> {
    if value.is_null() {
        return Err("null string argument".into());
    }
    Ok(unsafe { CStr::from_ptr(value) }.to_str()?.to_owned())
}

/// # Safety
/// Locator and optional strings are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_parquet_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_reader(
            locator,
            storage_options,
            scan_options,
            output,
            |locator, storage_options, scan_options| async move {
                let (operator, path) = storage::operator(&locator, &storage_options)?;
                let accessor = ParquetAccessor::new(OpenDalAccessor::new(operator));
                AsyncSource::new(accessor, ReadRequest::new(path).with_options(scan_options)).await
            },
        )
    }
}

/// # Safety
/// Source is live; output points to writable, uninitialized ArrowSchema storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_get_schema(
    source: *const DuckportSource,
    output: *mut FFI_ArrowSchema,
) -> *mut c_char {
    call(|| unsafe {
        if source.is_null() || output.is_null() {
            return Err("null schema argument".into());
        }
        ptr::write(
            output,
            FFI_ArrowSchema::try_from((*source).source.as_ref().unwrap().schema().as_ref())?,
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
        if source.is_null() || output.is_null() {
            return Err("null stream argument".into());
        }
        let handle = &*source;
        let source = handle.source.as_ref().unwrap();
        let reader = BatchReader {
            reader: Some(RUNTIME.block_on(source.reader())?),
            schema: source.schema(),
            done: false,
        };
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
        if output.is_null() {
            return Err("null source output".into());
        }
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
