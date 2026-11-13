use super::*;
use open_arrow_access::file_format::csv::{CsvDialect, CsvFileFormat, CsvInferenceOptions};
pub(super) async fn decode(
    input: Box<dyn ByteReader>,
    path: &str,
    options: &Options,
    read_options: ReadOptions,
) -> Result<Box<dyn RecordReader>> {
    let request = ReadRequest::new(path).with_options(read_options);
    Ok(CsvFileFormat::infer()
        .with_dialect(CsvDialect {
            delimiter: b',',
            ..Default::default()
        })
        .with_inference_options(CsvInferenceOptions {
            has_header: options.header,
            ..Default::default()
        })
        .open_input(input, request)
        .await?)
}
