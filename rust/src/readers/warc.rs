use crate::source::open_file_source;
use crate::storage;
use open_arrow_access::{OuterCompression, ReadRequest, file_format::warc::WarcArrowFormat};

crate::ffi::reader::export_reader!(duckport_warc_open;
    |locator, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let format = WarcArrowFormat;
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
