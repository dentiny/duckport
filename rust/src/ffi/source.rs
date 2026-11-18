use super::common::RUNTIME;
use crate::source::files::open as open_file_source;
use crate::{
    error::{Result, call, cleanup},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
    utils::{ffi_slice, string},
    validation,
};
use arrow_array::ffi_stream::FFI_ArrowArrayStream;
use arrow_schema::ffi::FFI_ArrowSchema;
use open_arrow_access::{
    BlockingRecordBatchReader, OuterCompression, ReadOptions, file_format::parquet::ParquetFormat,
};
use std::{ffi::c_char, ptr, sync::Arc};

pub struct DuckportSource {
    source: Option<AsyncSource>,
    metrics: crate::metrics::ReadMetrics,
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
        open_file_source(
            &locator,
            &storage_options,
            scan_options,
            ParquetFormat,
            OuterCompression::None,
        )
        .await
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
/// Source is live; columns contains column_count pointers to NUL-terminated UTF-8
/// names, borrowed only for this call. A zero count selects zero columns; columns
/// may then be null. Output points to writable, uninitialized stream storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_get_stream(
    source: *const DuckportSource,
    columns: *const *const c_char,
    column_count: usize,
    output: *mut FFI_ArrowArrayStream,
) -> *mut c_char {
    call(|| unsafe {
        let handle = validation::required(source.as_ref(), "null stream argument")?;
        validation::non_null(output, "null stream argument")?;
        let source = handle.source.as_ref().unwrap();
        let columns = ffi_slice(columns, column_count, "null projection columns")?;
        let projection = columns
            .iter()
            .map(|column| string(*column))
            .collect::<Result<Vec<_>>>()?;
        handle.metrics.begin_scan();
        let reader = handle
            .metrics
            .observe_reader(RUNTIME.block_on(source.reader(projection))?);
        let reader = BlockingRecordBatchReader::new(reader, Arc::clone(&RUNTIME));
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

/// # Safety
/// Source is live; output is writable. Called before starting its Arrow stream.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_configure_copy(
    source: *const DuckportSource,
    read_concurrency: usize,
    inspect_size: bool,
    bytes: *mut u64,
    known: *mut bool,
) -> *mut c_char {
    call(|| unsafe {
        let source = validation::required(source.as_ref(), "null source argument")?;
        validation::non_null(bytes, "null size output")?;
        validation::non_null(known, "null size output")?;
        *known = false;
        let source = source.source.as_ref().unwrap();
        if read_concurrency > 0 {
            source.set_read_concurrency(read_concurrency);
        }
        if inspect_size {
            match RUNTIME.block_on(source.statistics())?.source_bytes {
                open_arrow_access::Statistic::Exact(value)
                | open_arrow_access::Statistic::Estimated(value) => {
                    *bytes = value;
                    *known = true;
                }
                open_arrow_access::Statistic::Unknown => {}
            }
        }
        Ok(())
    })
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
        let mut storage = storage_options.read()?;
        storage.files = ffi_slice(
            scan_options.files,
            scan_options.file_count,
            "null selected files",
        )?
        .iter()
        .map(|file| {
            Ok(crate::options::SelectedFile {
                locator: string(file.locator)?,
                storage: file.storage.read()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
        let scan = scan_options.read()?;
        let metrics = storage.metrics.clone();
        let source = RUNTIME.block_on(open(locator, storage, scan))?;
        source.set_read_concurrency(scan_options.read_concurrency);
        *output = Box::into_raw(Box::new(DuckportSource {
            source: Some(source),
            metrics,
        }));
        Ok(())
    })
}

/// # Safety
/// Source is live; output points to writable metrics storage. No I/O is performed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_source_get_metrics(
    source: *const DuckportSource,
    output: *mut crate::metrics::DuckportReadMetrics,
) -> *mut c_char {
    call(|| unsafe {
        let source = validation::required(source.as_ref(), "null source")?;
        validation::non_null(output, "null metrics output")?;
        ptr::write(output, source.metrics.snapshot());
        Ok(())
    })
}
