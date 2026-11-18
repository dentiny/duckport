//! Lance dataset setup and backend-specific tests.
use super::{DuckportSink, open_writer};
use crate::{error::Result, options::StorageConfig, storage, utils, validation};
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{SinkExt, WriteOptions};
use open_arrow_sink_lance::LanceDatasetSink;
use open_arrow_sink_layer::{DownloadBlobLayer, FieldRemap, SchemaEvolutionLayer};
use open_arrow_sink_storage::StorageResolver;
use std::collections::HashSet;
use std::sync::Arc;

mod atif;
mod ffi;

struct BlobSource {
    document: String,
    storage: StorageConfig,
}

pub(crate) async fn open(
    path: &str,
    schema: SchemaRef,
    mode: &str,
    columns: HashSet<String>,
    config: &StorageConfig,
    options: WriteOptions,
    writer_count: usize,
    secrets: Vec<crate::security::ScopedHttpCredentials>,
    sources: Vec<BlobSource>,
) -> Result<DuckportSink> {
    let mut options = options.with_mode(utils::sink_mode(mode)?);
    let metrics = open_arrow_sink_core::IoMetrics::default();
    options.io_metrics = Some(metrics.clone().into());
    let paths = columns
        .iter()
        .map(|name| utils::resolve_column_path(&schema, name, "BLOB_COLUMNS"))
        .collect::<Result<Vec<_>>>()?;
    let (configured, automatic) = atif::configure(&schema)?;
    let mut paths = paths;
    paths.extend(
        automatic
            .iter()
            .map(|path| utils::resolve_column_path(&schema, path, "ATIF media"))
            .collect::<Result<Vec<_>>>()?,
    );
    let roots = paths.iter().map(|path| path[0].clone()).collect();
    validation::validate_blob_indexes(&options.indexes, &roots)?;
    let columns = paths.into_iter().map(|path| path.join(".")).collect();
    let destination = storage::sink_storage(path, config)?;
    let downloader = crate::blob_auth::blob_downloader(secrets, config.policy)?;
    let mut resolver = StorageResolver::default();
    for source in sources {
        let root = url::Url::parse(&source.document)?.join("/")?;
        resolver = resolver.with_storage(storage::sink_storage_from_root(root, &source.storage)?);
    }
    let remaps = schema
        .fields()
        .iter()
        .zip(configured.fields())
        .filter(|(a, b)| a != b)
        .map(|(input, target)| {
            let ty = target.data_type().clone();
            FieldRemap::new(input.name().clone(), target.clone(), move |array| {
                arrow_cast::cast(&array, &ty).map_err(open_arrow_sink_core::Error::backend)
            })
        })
        .collect();
    let sink = LanceDatasetSink::new(destination.storage)
        .with_source_resolver(Arc::new(resolver))
        .with_blob_downloader(std::sync::Arc::new(downloader))
        .layer(DownloadBlobLayer::new(columns))
        .layer(SchemaEvolutionLayer::new(remaps));
    let writer = open_writer(sink, destination.url, schema.clone(), options, writer_count).await?;
    Ok(DuckportSink {
        metrics,
        writer: Some(writer),
        schema,
        output_schema: None,
    })
}
