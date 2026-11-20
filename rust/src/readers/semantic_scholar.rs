use crate::service_files::{AttachmentSelection, impl_attachment_selection};
use crate::utils::optional_string;
use open_arrow_access::{
    AccessorExt, AttachmentFormat, ReadRequest,
    decompression::{DecompressionLayer, gzip::Gzip},
    service::semantic_scholar::{S2orcV2Accessor, SemanticScholarService},
};
use std::ffi::c_char;

#[repr(C)]
pub struct SemanticScholarOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub release: *const c_char,
}

impl_attachment_selection!(SemanticScholarOptions);

crate::ffi::reader::export_reader!(duckport_semantic_scholar_open, reader_options: SemanticScholarOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe { reader_options.read_attachment() }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://api.semanticscholar.org".into());
        let release = crate::validation::nonempty(
            unsafe { optional_string(reader_options.release) }?,
            "release is required",
        )?;
        let connection = crate::provider_auth::connection(
            &endpoint,
            &storage_options,
            crate::provider_auth::HttpAuth::ApiKeyHeader("x-api-key"),
        )?;
        let files = SemanticScholarService::with_connection(connection)
            .dataset(&release, &locator)
            .await?;
        if locator == "s2orc_v2"
            && matches!(attachment.options.format, AttachmentFormat::Json | AttachmentFormat::Jsonl)
        {
            // Semantic Scholar logical part names have no compression suffix.
            // Its S2ORC v2 release shards are gzip-compressed JSONL.
            let bytes = crate::metrics::ObservedBytes::new(files, storage_options.metrics.clone())
                .layer(DecompressionLayer::new(Gzip));
            return crate::source::DuckportSource::plannable(
                S2orcV2Accessor::new(bytes),
                scan_options.request(ReadRequest::new(attachment.file_name)),
            )
            .await;
        }
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
