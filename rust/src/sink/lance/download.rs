//! Source credentials take precedence over destination-owned download namespaces.
use futures::{TryStreamExt, stream};
use open_arrow_sink_core::{Error, async_trait};
use open_arrow_sink_storage::{
    BlobDownloader, DownloadContext, DownloadStream, Downloader, OpenDalDownloader, Storage, Url,
    opendal::Buffer,
};
use std::sync::Arc;

pub(super) fn with_sources(sources: Vec<Storage>) -> crate::error::Result<BlobDownloader> {
    let downloader = BlobDownloader::with_defaults()?;
    if sources.is_empty() {
        return Ok(downloader);
    }
    Ok(downloader.with_downloader(Arc::new(SourceDownloader {
        sources,
        fallback: OpenDalDownloader::new(Default::default())?,
    })))
}

struct SourceDownloader {
    sources: Vec<Storage>,
    fallback: OpenDalDownloader,
}

#[async_trait]
impl Downloader for SourceDownloader {
    async fn open(
        &self,
        source: &str,
        context: DownloadContext,
    ) -> open_arrow_sink_core::Result<DownloadStream> {
        let url = Url::parse(source).map_err(Error::backend)?;
        let selected = self
            .sources
            .iter()
            .find_map(|storage| storage.path(&url).ok().map(|path| (storage, path)));
        let Some((storage, path)) = selected else {
            return self.fallback.open(source, context).await;
        };
        let mut operator = storage.operator();
        if !storage
            .http_concurrency_limit()
            .shares_budget(context.http_requests())
        {
            let ctx = operator.base_context();
            let transport = context.http_requests().wrap(ctx.http_transport().clone());
            let ctx = ctx.with_http_transport(transport);
            operator = operator.with_context(ctx);
        }
        let reader = operator
            .reader(path.as_str())
            .await
            .map_err(Error::backend)?
            .into_stream(..)
            .await
            .map_err(Error::backend)?;
        let chunk_bytes = context.chunk_bytes();
        let chunks = stream::try_unfold(
            (reader, Buffer::default()),
            move |(mut reader, mut pending)| async move {
                loop {
                    if !pending.is_empty() {
                        let len = pending.len().min(chunk_bytes);
                        let chunk = pending.split_to(len).to_bytes();
                        return Ok(Some((chunk, (reader, pending))));
                    }
                    match reader.try_next().await.map_err(Error::backend)? {
                        Some(buffer) => pending = buffer,
                        None => return Ok(None),
                    }
                }
            },
        );
        Ok(Box::pin(chunks))
    }
}
