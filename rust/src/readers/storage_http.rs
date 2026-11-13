use super::*;
pub(super) async fn open(
    locator: &str,
    options: Options,
    read_options: ReadOptions,
) -> Result<Box<dyn RecordReader>> {
    if !(locator.starts_with("http://") || locator.starts_with("https://")) {
        return Err("source uses a different storage backend".into());
    }
    let format = options.format.as_deref().ok_or("format is required")?;
    let (input, path) = open_file(locator, &options).await?;
    decode_format(format, input, &path, &options, read_options).await
}
