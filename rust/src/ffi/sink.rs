//! Shared Arrow C ABI write options, write, commit and destruction.
use super::common::RUNTIME;
use crate::{
    error::{Result, call, cleanup},
    sink::DuckportSink,
    utils::{self, optional_string, string},
    validation,
};
use arrow_array::{
    RecordBatch, StructArray,
    ffi::{FFI_ArrowArray, from_ffi},
};
use arrow_schema::ffi::FFI_ArrowSchema;
use open_arrow_sink_core::{BlobHandleConfig, WriteOptions};
use std::ffi::c_char;

/// # Safety
/// Name is a borrowed NUL-terminated UTF-8 write-option name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_validate_byte_size(
    name: *const c_char,
    value: i64,
) -> *mut c_char {
    call(|| unsafe {
        utils::byte_size(&string(name)?, value)?;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn duckport_validate_write_concurrency(
    value: i64,
    preserve_order: bool,
) -> *mut c_char {
    call(|| validation::validate_write_concurrency(value, preserve_order))
}

#[unsafe(no_mangle)]
pub extern "C" fn duckport_validate_sample_percent(value: f64) -> *mut c_char {
    call(|| validation::sample_percent(value))
}

#[unsafe(no_mangle)]
pub extern "C" fn duckport_validate_sample_rows(value: i64, max_rows: u64) -> *mut c_char {
    call(|| validation::sample_rows(value, max_rows))
}

#[repr(C)]
pub struct DuckportColumnEncoding {
    pub column: *const c_char,
    pub compression: *const c_char,
    pub has_compression_level: i32,
    pub compression_level: i32,
    pub structural_encoding: *const c_char,
}

#[repr(C)]
pub struct DuckportIndexOptions {
    pub column: *const c_char,
    pub option: *const c_char,
}

/// Shared C ABI write options. -1 byte sizes and a null version preserve backend defaults.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DuckportWriteOptions {
    pub blob_inline_size_threshold: i64,
    pub blob_dedicated_size_threshold: i64,
    pub target_file_size: i64,
    pub storage_version: *const c_char,
    pub column_encodings: *const DuckportColumnEncoding,
    pub column_encoding_count: usize,
    pub indexes: *const DuckportIndexOptions,
    pub index_count: usize,
    pub writer_count: usize,
    pub source_bytes: u64,
    pub source_bytes_known: bool,
}

impl Default for DuckportWriteOptions {
    fn default() -> Self {
        Self {
            blob_inline_size_threshold: -1,
            blob_dedicated_size_threshold: -1,
            target_file_size: 512 * 1024 * 1024,
            storage_version: std::ptr::null(),
            column_encodings: std::ptr::null(),
            column_encoding_count: 0,
            indexes: std::ptr::null(),
            index_count: 0,
            writer_count: 1,
            source_bytes: 0,
            source_bytes_known: false,
        }
    }
}

impl DuckportWriteOptions {
    pub(crate) fn ingestion(&self) -> Result<open_arrow_sink_layer::ParallelIngestionLayer> {
        use open_arrow_sink_layer::{InputSizeHint, ParallelIngestionLayer};
        use std::num::NonZeroUsize;
        let writers = validation::required(
            NonZeroUsize::new(self.writer_count),
            "writer count must be positive",
        )?;
        if self.source_bytes_known && self.target_file_size > 0 {
            let target = utils::byte_size("target_file_size", self.target_file_size)?;
            Ok(
                ParallelIngestionLayer::new(NonZeroUsize::new(target).unwrap())
                    .with_input_size(InputSizeHint::SourceBytes(self.source_bytes))
                    .with_max_writers(writers)
                    .with_fallback_writers(writers),
            )
        } else {
            Ok(ParallelIngestionLayer::fixed(writers))
        }
    }
}

impl DuckportColumnEncoding {
    /// # Safety
    /// String pointers follow the same requirements as DuckportWriteOptions::convert.
    unsafe fn apply(
        &self,
        schema: &arrow_schema::Schema,
        options: &mut WriteOptions,
    ) -> Result<()> {
        let column = unsafe { string(self.column) }?;
        let path = utils::resolve_column_path(schema, &column, "Column encoding")?;
        let level = utils::boolean("has_compression_level", self.has_compression_level)?
            .then_some(self.compression_level);
        let codec = unsafe { optional_string(self.compression) }?;
        if let Some(compression) = utils::column_compression(&column, codec.as_deref(), level)? {
            if path.len() == 1 {
                options
                    .compression
                    .columns
                    .insert(path[0].clone(), compression);
            } else {
                options
                    .compression
                    .nested_columns
                    .insert(path.clone(), compression);
            }
        }
        if let Some(encoding) = unsafe { optional_string(self.structural_encoding) }? {
            let encoding = utils::column_structural_encoding(&column, &path, &encoding)?;
            options
                .compression
                .column_encodings
                .insert(path[0].clone(), encoding);
        }
        Ok(())
    }
}

impl DuckportWriteOptions {
    // Selecting URL columns requests blob support even without explicit layout overrides.
    /// # Safety
    /// Strings are null or borrowed NUL-terminated UTF-8. Encoding and index arrays
    /// contain the declared number of live records, or are null for zero records.
    pub(crate) unsafe fn convert(
        &self,
        blob_column_count: usize,
        schema: &arrow_schema::Schema,
    ) -> Result<WriteOptions> {
        let inline = utils::optional_byte_size(
            "BLOB_INLINE_SIZE_THRESHOLD",
            self.blob_inline_size_threshold,
        )?;
        let dedicated = utils::optional_byte_size(
            "BLOB_DEDICATED_SIZE_THRESHOLD",
            self.blob_dedicated_size_threshold,
        )?;
        let mut options = WriteOptions::default();
        options.storage_version = unsafe { optional_string(self.storage_version) }?;
        options.target_file_size =
            utils::optional_byte_size("TARGET_FILE_SIZE", self.target_file_size)?;
        if blob_column_count != 0 || inline.is_some() || dedicated.is_some() {
            options.blob_handle = Some(BlobHandleConfig {
                inline_size_threshold: inline,
                dedicated_size_threshold: dedicated.and_then(std::num::NonZeroUsize::new),
                ..Default::default()
            });
        }
        for entry in unsafe {
            utils::ffi_slice(
                self.column_encodings,
                self.column_encoding_count,
                "null column encodings",
            )
        }? {
            unsafe { entry.apply(schema, &mut options) }?;
        }
        for entry in
            unsafe { utils::ffi_slice(self.indexes, self.index_count, "null index options") }?
        {
            let column = unsafe { string(entry.column) }?;
            let option = unsafe { string(entry.option) }?;
            options
                .indexes
                .push(utils::index_spec(schema, &column, &option)?);
        }
        open_arrow_sink_core::validation::validate_indexes(schema, &options.indexes)?;
        Ok(options)
    }
}

/// # Safety
/// Sink is live. Array is an owned Arrow struct array; it is consumed even on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_write(
    sink: *mut DuckportSink,
    array: *mut FFI_ArrowArray,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(array, "null sink array")?;
        let array = FFI_ArrowArray::from_raw(array);
        let sink = validation::required(sink.as_mut(), "null sink")?;
        let schema = FFI_ArrowSchema::try_from(sink.schema.as_ref())?;
        let data = from_ffi(array, &schema)?;
        let batch = RecordBatch::from(StructArray::from(data)).with_schema(sink.schema.clone())?;
        RUNTIME.block_on(sink.write(batch))
    })
}
/// # Safety
/// Sink is a live pointer returned by a sink open function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_commit(sink: *mut DuckportSink) -> *mut c_char {
    call(|| unsafe { RUNTIME.block_on(validation::required(sink.as_mut(), "null sink")?.commit()) })
}
/// # Safety
/// Sink is null or a live pointer returned by a sink open function, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_destroy(sink: *mut DuckportSink) {
    cleanup(|| {
        if !sink.is_null() {
            let _guard = RUNTIME.enter();
            unsafe {
                drop(Box::from_raw(sink));
            }
        }
    });
}

#[repr(C)]
#[derive(Default)]
pub struct DuckportWriteMetrics {
    pub bytes_read: u64,
    pub bytes_written: u64,
}

/// # Safety
/// Sink is live; output points to writable metrics storage. Works after commit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_get_metrics(
    sink: *const DuckportSink,
    output: *mut DuckportWriteMetrics,
) -> *mut c_char {
    call(|| unsafe {
        let sink = validation::required(sink.as_ref(), "null sink")?;
        validation::non_null(output, "null metrics output")?;
        let snapshot = sink.metrics.snapshot();
        std::ptr::write(
            output,
            DuckportWriteMetrics {
                bytes_read: snapshot.bytes_read,
                bytes_written: snapshot.bytes_written,
            },
        );
        Ok(())
    })
}
