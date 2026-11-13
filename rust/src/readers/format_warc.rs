use super::*;
use open_arrow_access::file_format::warc::WarcArrowFormat;
pub(super) async fn decode(
    input: Box<dyn ByteReader>,
    path: &str,
    _options: &Options,
    read_options: ReadOptions,
) -> Result<Box<dyn RecordReader>> {
    let request = ReadRequest::new(path).with_options(read_options);
    Ok(WarcArrowFormat.open_input(input, request).await?)
}
