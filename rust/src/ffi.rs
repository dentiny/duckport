use crate::{
    error::{Result, call, cleanup},
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
    collections::HashMap,
    ffi::{CStr, c_char},
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::Arc,
};
use tokio::runtime::Runtime;

pub struct DuckportSource {
    source: Option<AsyncSource>,
    runtime: Arc<Runtime>,
}

impl Drop for DuckportSource {
    fn drop(&mut self) {
        let _guard = self.runtime.enter();
        self.source.take();
    }
}

struct BatchReader {
    reader: Option<Box<dyn RecordReader>>,
    runtime: Arc<Runtime>,
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
            self.runtime.block_on(self.reader.as_mut().unwrap().next())
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
        let _guard = self.runtime.enter();
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
/// Strings are NUL-terminated UTF-8; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_parquet_open(
    path: *const c_char,
    storage_options: *const c_char,
    batch_size: usize,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    call(|| unsafe {
        if output.is_null() {
            return Err("null source output".into());
        }
        *output = ptr::null_mut();
        let path = string(path)?;
        let config: HashMap<String, String> = serde_json::from_str(&string(storage_options)?)?;
        let batch_size = NonZeroUsize::new(batch_size)
            .filter(|s| s.get() <= 1048576)
            .ok_or("batch_size must be between 1 and 1048576")?;
        let (operator, path) = storage::operator(&path, config)?;
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        );
        let accessor = ParquetAccessor::new(OpenDalAccessor::new(operator));
        let request = ReadRequest::new(path).with_options(ReadOptions {
            batch_size,
            ..Default::default()
        });
        let source = runtime.block_on(AsyncSource::new(accessor, request))?;
        *output = Box::into_raw(Box::new(DuckportSource {
            source: Some(source),
            runtime,
        }));
        Ok(())
    })
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
            reader: Some(handle.runtime.block_on(source.reader())?),
            runtime: handle.runtime.clone(),
            schema: source.schema(),
            done: false,
        };
        ptr::write(output, FFI_ArrowArrayStream::new(Box::new(reader)));
        Ok(())
    })
}
/// # Safety
/// Source is null or a pointer returned by duckport_parquet_open, freed once.
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
