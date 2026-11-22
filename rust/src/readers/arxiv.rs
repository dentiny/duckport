mod metadata;

use crate::{source::DuckportSource as AsyncSource, utils::optional_string};
use open_arrow_access::{
    service::arxiv::{
        ArxivArticle, ArxivArticleReadRequest, ArxivContentFormat, ArxivReadRequest, ArxivService,
    },
    storage::OpenDalAccessor,
};
use std::{ffi::c_char, sync::LazyLock, time::Duration};

// Cloned operators and separate SQL scans share the official article budget.
static ARTICLE_REQUESTS: LazyLock<open_arrow_access::storage::HttpRequestRateLimitLayer> =
    LazyLock::new(|| {
        open_arrow_access::storage::HttpRequestRateLimitLayer::new(Duration::from_secs(3))
            .expect("valid arXiv request interval")
    });
#[repr(C)]
pub struct ArxivOptions {
    pub kind: *const c_char,
    pub format: *const c_char,
    pub article_id: *const c_char,
    pub month: *const c_char,
    pub shard: *const c_char,
    pub requester_pays: i32,
    pub metadata_uri: *const c_char,
    pub metadata_id_column: *const c_char,
    pub metadata_endpoint: *const c_char,
    pub metadata_storage: *const crate::options::DuckportStorageOptions,
}

crate::ffi::reader::export_reader!(duckport_arxiv_open, reader_options: ArxivOptions;
    |locator, storage_options, scan_options| async move {
        let metadata_endpoint = unsafe { optional_string(reader_options.metadata_endpoint) }?;
        let kind = unsafe { optional_string(reader_options.kind) }?;
        let format = unsafe { optional_string(reader_options.format) }?;
        let article_id = unsafe { optional_string(reader_options.article_id) }?;
        let month = unsafe { optional_string(reader_options.month) }?;
        let shard = unsafe { optional_string(reader_options.shard) }?;
        let metadata_uri = unsafe { optional_string(reader_options.metadata_uri) }?;
        let id_column = unsafe { optional_string(reader_options.metadata_id_column) }?;
        let requester_pays =
            crate::utils::boolean("requester_pays", reader_options.requester_pays)?;
        if format.is_some() && kind.is_some() {
            return Err("format and kind cannot be combined".into());
        }
        crate::validation::validate_arxiv_shard(shard.as_deref(), month.as_deref(), article_id.as_deref())?;
        if let Some(id) = article_id {
            if metadata_endpoint.is_some() {
                return Err("metadata_endpoint requires bulk content format".into());
            }
            if kind.is_some() || month.is_some() || metadata_uri.is_some() || id_column.is_some() || requester_pays {
                return Err("article_id cannot be combined with kind, month, metadata_uri, metadata_id_column, or requester_pays".into());
            }
            let format = crate::validation::arxiv_article_format(format.as_deref())?;
            let article = ArxivArticle::new(id, format);
            article.locator()?;
            let operator = crate::storage::directory_operator_with_requester_pays(
                &locator, &storage_options, false,
            )?;
            let official_http = url::Url::parse(&locator).is_ok_and(|url| {
                matches!(url.scheme(), "http" | "https")
                    && url.host_str().is_some_and(|host| {
                        host == "arxiv.org" || host.ends_with(".arxiv.org")
                    })
            });
            let operator = if official_http {
                operator.layer(ARTICLE_REQUESTS.clone())
            } else {
                operator
            };
            let storage = OpenDalAccessor::new(operator);
            return AsyncSource::plannable(
                ArxivService::new(storage).article_records(),
                scan_options.request(ArxivArticleReadRequest::new(vec![article])),
            ).await;
        }
        let format = crate::validation::arxiv_content_format(
            format.as_deref(), shard.as_deref(), metadata_uri.as_deref(), id_column.as_deref(),
        )?;
        let kind = match format {
            Some(ArxivContentFormat::Pdf) => "pdf",
            Some(ArxivContentFormat::Latex | ArxivContentFormat::Xml) => "source",
            None => kind.as_deref().unwrap_or("source"),
        };
        let kind = crate::utils::arxiv_kind(kind)?;
        crate::validation::validate_arxiv_metadata(kind, metadata_uri.as_deref(), id_column.as_deref())?;
        let month = month.map(|s| s.parse::<u16>()).transpose()?;
        let storage = OpenDalAccessor::new(crate::storage::directory_operator_with_requester_pays(
            &locator,
            &storage_options,
            requester_pays,
        )?);
        if metadata_endpoint.is_some() && format.is_none() {
            return Err("metadata_endpoint requires bulk content format".into());
        }
        let mut service = ArxivService::new(storage);
        if format.is_some() {
            let connection = open_arrow_access::service::common::Connection::new(
                metadata_endpoint.as_deref().unwrap_or("https://oaipmh.arxiv.org"),
            )?.with_http_config(storage_options.policy.config()?)?;
            service = service.with_metadata_connection(connection);
        }
        let records = service.records();
        let mut request = scan_options.request(ArxivReadRequest::new(kind));
        if let Some(month) = month {
            request = request.with_month(month);
        }
        if let Some(shard) = shard {
            request = request.with_shard(shard);
        }
        if let Some(format) = format {
            AsyncSource::plannable(records.content(format), request).await
        } else if let Some(uri) = metadata_uri {
            let config = crate::validation::required(
                unsafe { reader_options.metadata_storage.as_ref() },
                "metadata_uri requires its own storage configuration",
            )?;
            let mut config = unsafe { config.read() }?;
            config.metrics = storage_options.metrics.clone();
            let index = metadata::load(&uri, id_column.as_deref().unwrap_or("identifier"), &config).await?;
            AsyncSource::plannable(records.with_metadata(index), request).await
        } else {
            AsyncSource::plannable(records, request).await
        }
    }
);
