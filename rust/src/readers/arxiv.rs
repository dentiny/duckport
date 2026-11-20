mod metadata;

use crate::{source::DuckportSource as AsyncSource, utils::optional_string};
use open_arrow_access::{
    service::arxiv::{
        ArxivArticle, ArxivArticleFormat, ArxivArticleReadRequest, ArxivReadRequest, ArxivService,
    },
    storage::OpenDalAccessor,
};
use std::ffi::c_char;
#[repr(C)]
pub struct ArxivOptions {
    pub kind: *const c_char,
    pub format: *const c_char,
    pub article_id: *const c_char,
    pub month: *const c_char,
    pub requester_pays: i32,
    pub metadata_uri: *const c_char,
    pub metadata_id_column: *const c_char,
    pub metadata_storage: *const crate::options::DuckportStorageOptions,
}

crate::ffi::reader::export_reader!(duckport_arxiv_open, reader_options: ArxivOptions;
    |locator, storage_options, scan_options| async move {
        let kind = unsafe { optional_string(reader_options.kind) }?;
        let format = unsafe { optional_string(reader_options.format) }?;
        let article_id = unsafe { optional_string(reader_options.article_id) }?;
        let month = unsafe { optional_string(reader_options.month) }?;
        let metadata_uri = unsafe { optional_string(reader_options.metadata_uri) }?;
        let id_column = unsafe { optional_string(reader_options.metadata_id_column) }?;
        let requester_pays =
            crate::utils::boolean("requester_pays", reader_options.requester_pays)?;
        if format.is_some() && kind.is_some() {
            return Err("format and kind cannot be combined".into());
        }
        let article_format = format.as_deref().map(|format| match format {
            "pdf" => Ok(ArxivArticleFormat::Pdf),
            "latex" => Ok(ArxivArticleFormat::Latex),
            "html" => Ok(ArxivArticleFormat::Html),
            _ => Err("format must be pdf, latex, or html"),
        }).transpose()?;
        if let Some(id) = article_id {
            if kind.is_some() || month.is_some() || metadata_uri.is_some() || id_column.is_some() || requester_pays {
                return Err("article_id cannot be combined with kind, month, metadata_uri, metadata_id_column, or requester_pays".into());
            }
            let format = article_format.ok_or("article_id requires format (pdf, latex, or html)")?;
            let article = ArxivArticle::new(id, format);
            article.locator()?;
            let storage = OpenDalAccessor::new(crate::storage::directory_operator_with_requester_pays(
                &locator, &storage_options, false,
            )?);
            return AsyncSource::plannable(
                ArxivService::new(storage).article_records(),
                scan_options.request(ArxivArticleReadRequest::new(vec![article])),
            ).await;
        }
        let kind = match article_format {
            Some(ArxivArticleFormat::Pdf) => "pdf",
            Some(ArxivArticleFormat::Latex) => "source",
            Some(ArxivArticleFormat::Html) => return Err("format=html requires article_id; arXiv has no HTML bulk manifest".into()),
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
        let records = ArxivService::new(storage).records();
        let mut request = scan_options.request(ArxivReadRequest::new(kind));
        if let Some(month) = month {
            request = request.with_month(month);
        }
        if let Some(uri) = metadata_uri {
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
