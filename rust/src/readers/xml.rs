use crate::source::files::open as open_file_source;
use crate::utils::optional_string;
use open_arrow_access::{OuterCompression, file_format::xml::XmlFormat};
use std::ffi::c_char;

#[repr(C)]
pub struct XmlOptions {
    pub record_path: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_xml_open, reader_options: XmlOptions;
    |locator, storage_options, scan_options| async move {
        let record_path = crate::validation::required(
            unsafe { optional_string(reader_options.record_path) }?,
            "record_path is required",
        )?;

        let format = XmlFormat::new(record_path);
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
