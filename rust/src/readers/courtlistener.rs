use crate::{source::DuckportSource as AsyncSource, utils::optional_string};
use open_arrow_access::{
    ReadRequest,
    file_format::{
        FileAccessor,
        csv::{CsvDialect, CsvFileFormat, CsvHeader, CsvSchemaMode},
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

crate::ffi::reader::export_reader!(duckport_courtlistener_open, reader_options: CourtlistenerOptions;
    |locator, storage_options, scan_options| async move {
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
        let format = CsvFileFormat::infer()
            .with_schema_mode(CsvSchemaMode::AllStrings)
            .with_dialect(CsvDialect {
                header: CsvHeader::Present,
                ..Default::default()
            });
        AsyncSource::plannable(
            FileAccessor::from_format(storage, format),
            scan_options.request(ReadRequest::new(snapshot.object())),
        )
        .await
    }
);
