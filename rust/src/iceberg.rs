//! REST catalog setup and the Iceberg adapter for the shared COPY writer.
use crate::{error::Result, options::StorageConfig, sink::DuckportSink, storage, utils};
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
    crate::validation::validate_uri(&uri)?;
    if !matches!(uri.scheme(), "http" | "https") {
        return Err("CATALOG_URI must use http:// or https://".into());
    }
    let identifier = identifier.split('.').collect::<Vec<_>>();
    if identifier.len() < 2 || identifier.iter().any(|part| part.is_empty()) {
        return Err("Iceberg destination must be namespace.table".into());
    }
    let identifier = TableIdent::from_strs(identifier)?;
    let root = utils::lance_url(storage_root)?;
    if root.scheme() == "s3" && root.path() != "/" {
        return Err("Iceberg STORAGE_ROOT must be an S3 bucket root (s3://bucket/)".into());
    }
    // A trailing slash selects the directory itself as the operator namespace.
    let operator = if root.scheme() == "file" {
        storage::create_local_directory_operator(&utils::local_path(root.as_str())?)?
    } else {
        storage::operator(root.as_str(), config)?.0
    };
    let mut scope = root;
    if scope.scheme() == "s3" {
        scope.set_path("/");
    }
    let factory = OpenDalStorageFactory::new(Storage::new(operator, scope)?);
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
    if !table
        .metadata()
        .default_partition_spec()
        .fields()
        .is_empty()
    {
        return Err("Iceberg COPY currently supports unpartitioned tables only".into());
    }
    let output_schema = Arc::new(iceberg::arrow::schema_to_arrow_schema(
        table.metadata().current_schema(),
    )?);
    // DuckDB does not export Iceberg field IDs. Match names and types first,
    // then attach the catalog schema, including IDs and requiredness.
    if schema.fields().len() != output_schema.fields().len()
        || schema
            .fields()
            .iter()
            .zip(output_schema.fields())
            .any(|(input, output)| {
                input.name() != output.name()
                    || !input.data_type().equals_datatype(output.data_type())
            })
    {
        return Err(
            "Iceberg input column names, order and types must match the existing table".into(),
        );
    }
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
