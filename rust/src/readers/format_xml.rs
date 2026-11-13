use super::*;
use open_arrow_access::file_format::xml::XmlFormat;
pub(super) async fn decode(
    input: Box<dyn ByteReader>,
    path: &str,
    options: &Options,
    read_options: ReadOptions,
) -> Result<Box<dyn RecordReader>> {
    let request = ReadRequest::new(path).with_options(read_options);
    Ok(XmlFormat::new(
        options
            .record_path
            .as_deref()
            .ok_or("record_path is required")?,
    )
    .open_input(input, request)
    .await?)
}
