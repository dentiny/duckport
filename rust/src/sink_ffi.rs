use crate::{
    error::{call, cleanup},
    ffi::string,
    sink::DuckportSink,
};
use arrow_array::{
    RecordBatch, StructArray,
    ffi::{FFI_ArrowArray, from_ffi},
};
use arrow_schema::{Schema, ffi::FFI_ArrowSchema};
use std::{ffi::c_char, ptr, sync::Arc};

/// # Safety
/// Strings are NUL-terminated UTF-8, schema is live and borrowed, output is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_lance_open(
    path: *const c_char,
    schema: *const FFI_ArrowSchema,
    mode: *const c_char,
    columns: *const *const c_char,
    column_count: usize,
    storage: *const crate::options::DuckportStorageOptions,
    output: *mut *mut DuckportSink,
) -> *mut c_char {
    call(|| unsafe {
        if output.is_null() {
            return Err("null sink output".into());
        }
        *output = ptr::null_mut();
        if schema.is_null() {
            return Err("null sink schema".into());
        }
        if storage.is_null() || (column_count > 0 && columns.is_null()) {
            return Err("null Lance options".into());
        }
        let columns = if column_count == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(columns, column_count)
        };
        let columns = columns
            .iter()
            .map(|column| string(*column))
            .collect::<crate::error::Result<std::collections::HashSet<_>>>()?;
        let storage = (*storage).read()?;
        let mut config = std::collections::HashMap::new();
        for (key, value) in [
            ("endpoint", storage.endpoint),
            ("region", storage.region),
            ("access_key_id", storage.access_key_id),
            ("secret_access_key", storage.secret_access_key),
            ("session_token", storage.session_token),
        ] {
            if let Some(value) = value {
                config.insert(key.to_string(), value);
            }
        }
        let path = string(path)?;
        if path.starts_with("s3://") {
            config.insert("disable_config_load".into(), "true".into());
            config.insert("disable_ec2_metadata".into(), "true".into());
            config.insert(
                "enable_virtual_host_style".into(),
                storage.virtual_host_style.to_string(),
            );
            config.insert(
                "allow_anonymous".into(),
                storage.allow_anonymous.to_string(),
            );
        }
        let sink = DuckportSink::new(
            &path,
            Arc::new(Schema::try_from(&*schema)?),
            &string(mode)?,
            columns,
            config,
        )?;
        *output = Box::into_raw(Box::new(sink));
        Ok(())
    })
}
/// # Safety
/// Sink is live. Array is an owned Arrow struct array; it is consumed even on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_write(
    sink: *mut DuckportSink,
    array: *mut FFI_ArrowArray,
) -> *mut c_char {
    call(|| unsafe {
        if array.is_null() {
            return Err("null sink array".into());
        }
        let array = FFI_ArrowArray::from_raw(array);
        if sink.is_null() {
            return Err("null sink".into());
        }
        let schema = FFI_ArrowSchema::try_from((*sink).schema.as_ref())?;
        let data = from_ffi(array, &schema)?;
        let batch = RecordBatch::from(StructArray::from(data));
        // StructArray conversion may drop root schema metadata; retain the fixed schema.
        (*sink).write(RecordBatch::try_new(
            (*sink).schema.clone(),
            batch.columns().to_vec(),
        )?)
    })
}
/// # Safety
/// Sink is a live pointer returned by duckport_lance_open.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_commit(sink: *mut DuckportSink) -> *mut c_char {
    call(|| unsafe {
        if sink.is_null() {
            return Err("null sink".into());
        }
        (*sink).commit()
    })
}
/// # Safety
/// Sink is null or a live pointer returned by duckport_lance_open, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_sink_destroy(sink: *mut DuckportSink) {
    cleanup(|| {
        if !sink.is_null() {
            unsafe {
                drop(Box::from_raw(sink));
            }
        }
    });
}
