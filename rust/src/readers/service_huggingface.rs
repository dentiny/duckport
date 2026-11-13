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
    let mut request = HuggingFaceDatasetRequest::new(
        locator,
        options.config.as_deref().unwrap_or("default"),
        options.split.as_deref().unwrap_or("train"),
    )
    .with_options(read_options);
    if let Some(revision) = options.revision {
        request = request.with_revision(revision);
    }
    if options.allow_partial {
        request = request.with_partial_exports();
    }
    Ok(service.dataset(request).await?)
}
