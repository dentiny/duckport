use crate::utils::optional_string;
use open_arrow_access::{
    AccessorExt, ReadRequest,
    decompression::{DecompressionLayer, gzip::Gzip},
    service::semantic_scholar::{S2orcV2Accessor, SemanticScholarService},
};
use std::ffi::c_char;

#[repr(C)]
pub struct SemanticScholarOptions {
    pub file_name: *const c_char,
    pub endpoint: *const c_char,
    pub release: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_semantic_scholar_open, reader_options: SemanticScholarOptions;
    |locator, storage_options, scan_options| async move {
        crate::validation::validate_semantic_scholar_locator(&locator)?;
        let file_name = crate::validation::nonempty(
            unsafe { optional_string(reader_options.file_name) }?,
            "file_name is required",
        )?;
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
        // Logical shard names have no suffix; S2ORC v2 is always gzip JSONL.
        let bytes = crate::metrics::ObservedBytes::new(files, storage_options.metrics.clone())
            .layer(DecompressionLayer::new(Gzip));
        crate::source::DuckportSource::plannable(
            S2orcV2Accessor::new(bytes),
            scan_options.request(ReadRequest::new(file_name)),
        )
        .await
    }
);
