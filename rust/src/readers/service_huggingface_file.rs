use super::*;
use open_arrow_access::service::huggingface::*;
pub(super) async fn open(
    locator: &str,
    options: Options,
    read_options: ReadOptions,
) -> Result<Box<dyn RecordReader>> {
    let mut settings = HuggingFaceOptions::default();
    if let Some(endpoint) = &options.endpoint {
        settings = settings.with_endpoint(endpoint);
    }
    if let Some(token) = &options.token {
        settings = settings.with_token(token);
    }
    let service = HuggingFaceService::new(settings).await?;
    let mut repository = HuggingFaceRepository::new(locator);
    if let Some(revision) = &options.revision {
        repository = repository.with_revision(revision);
    }
    let path = options.path.as_deref().ok_or("path is required")?;
    let files = service.repository(repository).await?;
    let input = files.open(ByteReadRequest::new(path)).await?;
    decode_format(
        options.format.as_deref().ok_or("format is required")?,
        input,
        path,
        &options,
        read_options,
    )
    .await
}
