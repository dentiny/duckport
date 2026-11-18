use crate::source::files::open as open_file_source;
use open_arrow_access::{OuterCompression, file_format::warc::WarcArrowFormat};

crate::ffi::reader::export_reader!(duckport_warc_open;
    |locator, storage_options, scan_options| async move {
        let format = WarcArrowFormat;
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
