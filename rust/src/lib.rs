mod source;

use arrow_array::{RecordBatch, RecordBatchReader, ffi_stream::FFI_ArrowArrayStream};
use arrow_schema::{ArrowError, SchemaRef};
use open_arrow_access::RecordReader;

use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
};
use tokio::runtime::Runtime;

struct BatchReader {
    // Drop the source before shutting down its runtime, including on early LIMIT.
    reader: Option<Box<dyn RecordReader>>,
    runtime: Runtime,
    schema: SchemaRef,
    done: bool,
}
impl BatchReader {
    fn open(locator: &str, reader: &str, options: &str, batch_size: usize) -> Result<Self, String> {
        let options: source::Options =
            serde_json::from_str(options).map_err(|e| format!("invalid reader options: {e}"))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let reader = runtime
            .block_on(source::open(reader, locator, options, batch_size))
            .map_err(|e| e.to_string())?;
        let schema = reader.metadata().schema.clone();
        Ok(Self {
            reader: Some(reader),
            runtime,
            schema,
            done: false,
        })
    }
}
impl Iterator for BatchReader {
    type Item = Result<RecordBatch, ArrowError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        // No Rust panic may cross Arrow's C callback boundary.
        match catch_unwind(AssertUnwindSafe(|| {
            self.runtime.block_on(self.reader.as_mut().unwrap().next())
        })) {
            Ok(Ok(Some(batch))) => Some(Ok(batch)),
            Ok(Ok(None)) => {
                self.done = true;
                None
            }
            Ok(Err(error)) => {
                self.done = true;
                Some(Err(ArrowError::ExternalError(Box::new(error))))
            }
            Err(_) => {
                self.done = true;
                Some(Err(ArrowError::ComputeError(
                    "source reader panicked".into(),
                )))
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

/// Open an owned Arrow C stream. On failure, return a Rust-owned error string.
/// The caller must release successful streams and free errors with duckport_source_error_free.
///
/// # Safety
/// String pointers must be non-null, NUL-terminated UTF-8. Output points to writable,
/// uninitialized ArrowArrayStream storage. No pointers are retained after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_open(
    locator: *const c_char,
    reader: *const c_char,
    options: *const c_char,
    batch_size: usize,
    out: *mut FFI_ArrowArrayStream,
) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        if locator.is_null() || reader.is_null() || options.is_null() || out.is_null() {
            return Err("null source argument".into());
        }
        // SAFETY: Required by the documented C ABI contract; outputs are checked above.
        let (locator, reader, options) = unsafe {
            (
                CStr::from_ptr(locator),
                CStr::from_ptr(reader),
                CStr::from_ptr(options),
            )
        };
        let reader = BatchReader::open(
            locator.to_str().map_err(|e| e.to_string())?,
            reader.to_str().map_err(|e| e.to_string())?,
            options.to_str().map_err(|e| e.to_string())?,
            batch_size,
        )?;
        // SAFETY: The caller supplies writable space for an ArrowArrayStream.
        unsafe {
            ptr::write(out, FFI_ArrowArrayStream::new(Box::new(reader)));
        }
        Ok(())
    }));
    match result {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(error)) => CString::new(error.replace('\0', " ")).unwrap().into_raw(),
        Err(_) => CString::new("source initialization panicked")
            .unwrap()
            .into_raw(),
    }
}
/// # Safety
/// error must be null or a pointer returned by duckport_source_open, freed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_error_free(error: *mut c_char) {
    if !error.is_null() {
        unsafe {
            drop(CString::from_raw(error));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn csv_streams_in_batches_and_keeps_nulls() {
        let mut file = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
        write!(file, "id,name\n1,Alice\n2,\n3,Charlie\n").unwrap();
        let reader = BatchReader::open(
            file.path().to_str().unwrap(),
            "csv",
            r#"{"header":true}"#,
            2,
        )
        .unwrap();
        let batches = reader.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(batches.iter().map(|b| b.num_rows()).sum::<usize>(), 3);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].column(1).null_count(), 1);
    }
    #[test]
    fn explicit_json_format_without_extension() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"id":1,"nested":{{"ok":true}},"tags":["a","b"]}}"#
        )
        .unwrap();
        let mut reader = BatchReader::open(file.path().to_str().unwrap(), "json", "{}", 1).unwrap();
        assert_eq!(reader.next().unwrap().unwrap().num_columns(), 3);
        assert!(reader.next().is_none());
    }
    #[test]
    fn rejects_invalid_options_and_missing_files() {
        for (format, options) in [
            ("csv", r#"{"unknown":1}"#),
            ("csv", "not json"),
            ("json", "{}"),
            ("csv", r#"{"storage_options":{"root":"/tmp"}}"#),
        ] {
            assert!(BatchReader::open("/no-such-duckport-file", format, options, 10).is_err());
        }
        assert!(BatchReader::open("anything", "csv", "{}", 0).is_err());
    }
    #[test]
    fn ffi_errors_and_early_stream_release() {
        unsafe {
            let name = CString::new("missing").unwrap();
            let format = CString::new("json").unwrap();
            let opts = CString::new("{}").unwrap();
            let mut stream = std::mem::MaybeUninit::uninit();
            let error = duckport_source_open(
                name.as_ptr(),
                format.as_ptr(),
                opts.as_ptr(),
                10,
                stream.as_mut_ptr(),
            );
            assert!(!error.is_null());
            duckport_source_error_free(error);
            let mut file = tempfile::Builder::new()
                .suffix(".jsonl")
                .tempfile()
                .unwrap();
            writeln!(file, r#"{{"id":1}}"#).unwrap();
            let name = CString::new(file.path().to_str().unwrap()).unwrap();
            assert!(
                duckport_source_open(
                    name.as_ptr(),
                    format.as_ptr(),
                    opts.as_ptr(),
                    10,
                    stream.as_mut_ptr()
                )
                .is_null()
            );
            drop(stream.assume_init());
        }
    }
}
