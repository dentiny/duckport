use crate::source::files::open as open_file_source;
use open_arrow_access::{
    OuterCompression,
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

        let format = CsvFileFormat::infer().with_dialect(CsvDialect {
            header,
            delimiter: reader_options.delimiter,
            ..Default::default()
        });
        open_file_source(
            &locator,
            &storage_options,
            scan_options,
            format,
            OuterCompression::Auto,
        )
        .await
    }
);
