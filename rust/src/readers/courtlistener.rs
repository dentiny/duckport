use crate::{
    ffi::source::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig},
    source::DuckportSource as AsyncSource,
    utils::optional_string,
};
use open_arrow_access::{
    Accessor, ByteReadRequest, ReadRequest,
    file_format::{
        FileAccessor,
        csv::{CsvDialect, CsvFileFormat, CsvHeader},
    },
    service::courtlistener::{CourtListenerBulkAccessor, CourtListenerSnapshot},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

#[repr(C)]
pub struct CourtlistenerOptions {
    pub table_name: *const c_char,
    pub date: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_courtlistener_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &CourtlistenerOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let table_name = crate::validation::nonempty(
            unsafe { optional_string(reader_options.table_name) }?,
            "table_name is required",
        )?;
        let date = crate::validation::nonempty(
            unsafe { optional_string(reader_options.date) }?,
            "date is required",
        )?;
        let snapshot = CourtListenerSnapshot::new(table_name, date)?;
        let operator = crate::storage::directory_operator(&locator, &storage_options)?;
        let storage = CourtListenerBulkAccessor::new(OpenDalAccessor::new(operator));
        // CourtListener exports contain all-null columns. Match the upstream tests:
        // retain every column as VARCHAR and let SQL apply table-specific casts.
        let input = storage
            .open(ByteReadRequest::new(snapshot.object()))
            .await?;
        let mut header_reader = csv_async::AsyncReaderBuilder::new().create_reader(
            open_arrow_access::into_async_read(input, "read_courtlistener_header"),
        );
        let header = header_reader.headers().await?.clone();
        let schema = crate::utils::courtlistener_header(&header)?;
        let format = CsvFileFormat::new(std::sync::Arc::new(schema)).with_dialect(CsvDialect {
            header: CsvHeader::Present,
            ..Default::default()
        });
        AsyncSource::new(
            FileAccessor::from_format(storage, format),
            ReadRequest::new(snapshot.object()).with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
