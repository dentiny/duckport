use crate::source::open_file_source;
use crate::{storage, utils::optional_string};
use open_arrow_access::{OuterCompression, ReadRequest, file_format::xml::XmlFormat};
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

        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let format = XmlFormat::new(record_path);
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
