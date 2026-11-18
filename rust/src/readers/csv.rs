use crate::source::open_file_source;
use crate::storage;
use open_arrow_access::{
    OuterCompression, ReadRequest,
    file_format::csv::{CsvDialect, CsvFileFormat, CsvHeader},
};

#[repr(C)]
pub struct CsvOptions {
    /// -1 infers the header; 0 disables it; 1 enables it.
    pub header: i32,
    pub delimiter: u8,
}

crate::ffi::reader::export_reader!(duckport_csv_open, reader_options: CsvOptions;
    |locator, storage_options, scan_options| async move {
        let header = match reader_options.header {
            -1 => CsvHeader::Auto,
            value => CsvHeader::from(crate::utils::boolean("header", value)?),
        };

        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let format = CsvFileFormat::infer().with_dialect(CsvDialect {
            header,
            delimiter: reader_options.delimiter,
            ..Default::default()
        });
        Ok(open_file_source(
            operator,
            ReadRequest::new(path).with_options(scan_options),
            format,
            OuterCompression::Auto,
            &storage_options.metrics,
        )
        .await?)
    }
);
