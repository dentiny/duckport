//! REST catalog setup and the Iceberg adapter for the shared COPY writer.
use crate::{
    error::Result, options::StorageConfig, sink::DuckportSink, storage, utils, validation,
};
use arrow_schema::SchemaRef;
use iceberg_catalog_rest::{
    REST_CATALOG_PROP_URI, REST_CATALOG_PROP_WAREHOUSE, RestCatalogBuilder,
};
use open_arrow_sink_core::{Ordering, Sink, WriteMode, WriteOptions};
use open_arrow_sink_iceberg::{
    IcebergTableSink, OpenDalStorageFactory, Storage,
    iceberg::{self, Catalog, CatalogBuilder, TableIdent},
};
use std::{collections::HashMap, sync::Arc};

pub(crate) async fn open(
    identifier: &str,
    schema: SchemaRef,
    catalog_uri: &str,
    warehouse: &str,
    storage_root: &str,
    config: &StorageConfig,
) -> Result<DuckportSink> {
    let uri = url::Url::parse(catalog_uri)?;
    validation::validate_iceberg_catalog_uri(&uri)?;
    let identifier = identifier.split('.').collect::<Vec<_>>();
    validation::validate_iceberg_identifier(&identifier)?;
    let identifier = TableIdent::from_strs(identifier)?;
    let root = utils::sink_url(storage_root)?;
    validation::validate_iceberg_storage_root(&root)?;
    let operator = storage::directory_operator(root.as_str(), config)?;
    let factory = OpenDalStorageFactory::new(Storage::new(operator, root)?);
    let mut props = HashMap::from([(REST_CATALOG_PROP_URI.into(), uri.to_string())]);
    if !warehouse.is_empty() {
        props.insert(REST_CATALOG_PROP_WAREHOUSE.into(), warehouse.into());
    }
    if let Ok(token) = std::env::var("DUCKPORT_ICEBERG_TOKEN") {
        props.insert("token".into(), token);
    }
    let catalog = RestCatalogBuilder::default()
        .with_storage_factory(Arc::new(factory))
        .load("duckport", props)
        .await?;
    open_catalog(Arc::new(catalog), identifier, schema).await
}

pub(crate) async fn open_catalog(
    catalog: Arc<dyn Catalog>,
    identifier: TableIdent,
    schema: SchemaRef,
) -> Result<DuckportSink> {
    let table = catalog.load_table(&identifier).await?;
    validation::validate_iceberg_table(&table)?;
    let output_schema = Arc::new(iceberg::arrow::schema_to_arrow_schema(
        table.metadata().current_schema(),
    )?);
    // DuckDB does not export Iceberg field IDs. Match names and types first,
    // then attach the catalog schema, including IDs and requiredness.
    validation::validate_iceberg_schema(&schema, &output_schema)?;
    let writer = IcebergTableSink::new(catalog)
        .open(
            identifier,
            output_schema.clone(),
            WriteOptions::default()
                .with_mode(WriteMode::Append)
                .with_ordering(Ordering::Unordered),
        )
        .await?;
    Ok(DuckportSink {
        writer: Some(writer),
        schema,
        output_schema: Some(output_schema),
    })
}

mod ffi;
#[cfg(test)]
mod tests;
