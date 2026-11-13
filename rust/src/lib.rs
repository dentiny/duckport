mod storage;

use arrow_array::{RecordBatch, RecordBatchReader, ffi_stream::FFI_ArrowArrayStream};
use arrow_schema::{ArrowError, SchemaRef, ffi::FFI_ArrowSchema};
use open_arrow_access::{
    Accessor, ReadOptions, ReadRequest, RecordReader, file_format::parquet::ParquetAccessor,
    storage::OpenDalAccessor,
};
use std::{
    collections::HashMap,
    error::Error,
    ffi::{CStr, CString, c_char},
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::{Arc, Mutex},
};
use tokio::runtime::Runtime;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub struct DuckportSource {
    runtime: Arc<Runtime>,
    schema: SchemaRef,
    accessor: ParquetAccessor<OpenDalAccessor>,
    request: ReadRequest,
    bound: Mutex<Option<Box<dyn RecordReader>>>,
}
impl DuckportSource {
    fn new(
        operator: open_arrow_access::storage::opendal::Operator,
        path: String,
        batch_size: NonZeroUsize,
    ) -> Result<Self> {
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
        let reader = runtime.block_on(accessor.open(request.clone()))?;
        let schema = reader.metadata().schema.clone();
        Ok(Self {
            runtime,
            schema,
            accessor,
            request,
            bound: Mutex::new(Some(reader)),
        })
    }
    fn reader(&self) -> Result<BatchReader> {
        let reader = match self
            .bound
            .lock()
            .map_err(|_| "source lock poisoned")?
            .take()
        {
            Some(reader) => reader,
            None => self
                .runtime
                .block_on(self.accessor.open(self.request.clone()))?,
        };
        if reader.metadata().schema != self.schema {
            return Err("source schema changed; rebind the query".into());
        }
        Ok(BatchReader {
            reader: Some(reader),
            runtime: self.runtime.clone(),
            schema: self.schema.clone(),
            done: false,
        })
    }
}
impl Drop for DuckportSource {
    fn drop(&mut self) {
        let _guard = self.runtime.enter();
        if let Ok(reader) = self.bound.get_mut() {
            reader.take();
        }
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
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(self.reader.as_mut().unwrap().next())
        }));
        match result {
            Ok(Ok(batch)) => {
                self.done = batch.is_none();
                batch.map(Ok)
            }
            error => {
                self.done = true;
                Some(Err(match error {
                    Ok(Err(error)) => ArrowError::ExternalError(Box::new(error)),
                    _ => ArrowError::ComputeError("source reader panicked".into()),
                }))
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

fn call(operation: impl FnOnce() -> Result<()>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => ptr::null_mut(),
        result => {
            let message = match result {
                Ok(Err(error)) => error.to_string(),
                _ => "source operation panicked".into(),
            };
            CString::new(message.replace('\0', " ")).unwrap().into_raw()
        }
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
        let source = DuckportSource::new(operator, path, batch_size)?;
        *output = Box::into_raw(Box::new(source));
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
            FFI_ArrowSchema::try_from((*source).schema.as_ref())?,
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
        ptr::write(
            output,
            FFI_ArrowArrayStream::new(Box::new((*source).reader()?)),
        );
        Ok(())
    })
}
/// # Safety
/// Source is null or a pointer returned by duckport_parquet_open, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_destroy(source: *mut DuckportSource) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !source.is_null() {
            unsafe {
                drop(Box::from_raw(source));
            }
        }
    }));
}
/// # Safety
/// Error is null or a pointer returned by these FFI functions, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_error_free(error: *mut c_char) {
    if !error.is_null() {
        unsafe {
            drop(CString::from_raw(error));
        }
    }
}
