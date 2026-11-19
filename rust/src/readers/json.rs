use crate::source::files::open as open_file_source;
use open_arrow_access::{OuterCompression, file_format::json::JsonFormat};

crate::ffi::reader::export_reader!(duckport_json_open;
    |locator, storage_options, scan_options| async move {
        let format = JsonFormat::infer();
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
