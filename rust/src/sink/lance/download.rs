//! Small protocol adapter; BlobDownloader owns scheduling and memory admission.
use base64::{Engine, engine::general_purpose::STANDARD};
use open_arrow_sink_core::{Error, Result, async_trait};
use open_arrow_sink_storage::{DownloadContext, DownloadStream, Downloader, OpenDalDownloader};

pub(super) struct MediaDownloader {
    pub max_data_bytes: usize,
    pub origins: Vec<url::Url>,
    pub fallback: OpenDalDownloader,
}

#[async_trait]
impl Downloader for MediaDownloader {
    async fn open(&self, source: &str, context: DownloadContext) -> Result<DownloadStream> {
        if source.starts_with("data:") {
            let bytes = decode_data_uri(source, self.max_data_bytes)?;
            let size = context.chunk_bytes();
            return Ok(Box::pin(futures::stream::unfold(
                (bytes, 0),
                move |(bytes, offset)| async move {
                    if offset == bytes.len() {
                        return None;
                    }
                    let end = (offset + size).min(bytes.len());
                    let chunk = open_arrow_access::Bytes::copy_from_slice(&bytes[offset..end]);
                    Some((Ok(chunk), (bytes, end)))
                },
            )));
        }
        let url = url::Url::parse(source).map_err(Error::backend)?;
        if !self.origins.iter().any(|origin| {
            origin.scheme() == url.scheme()
                && origin.host_str() == url.host_str()
                && origin.port_or_known_default() == url.port_or_known_default()
        }) {
            return self.fallback.open(source, context).await;
        }
        if matches!(url.scheme(), "http" | "https") {
            let mut scope = url.clone();
            scope.set_query(None);
            scope.set_fragment(None);
            let source = context.resolve_source(&scope)?;
            return OpenDalDownloader::new(Default::default())?
                .with_http_storage(source.storage)?
                .open(url.as_str(), context)
                .await;
        }
        self.fallback.open(source, context).await
    }
}

fn decode_data_uri(uri: &str, limit: usize) -> Result<Vec<u8>> {
    let (header, encoded) = uri
        .split_once(',')
        .ok_or_else(|| Error::InvalidInput("invalid media data URI".into()))?;
    let decoded = percent_encoding::percent_decode_str(encoded);
    let len = decoded.clone().count();
    let is_base64 = header.ends_with(";base64");
    let max_output = if is_base64 {
        len.div_ceil(4).saturating_mul(3).saturating_sub(
            decoded
                .clone()
                .skip(len.saturating_sub(2))
                .filter(|b| *b == b'=')
                .count(),
        )
    } else {
        len
    };
    if max_output > limit {
        return Err(Error::InvalidInput(
            "media data URI exceeds download payload limit".into(),
        ));
    }
    let bytes = decoded.collect::<Vec<_>>();
    if is_base64 {
        STANDARD.decode(bytes).map_err(Error::backend)
    } else {
        Ok(bytes)
    }
}
