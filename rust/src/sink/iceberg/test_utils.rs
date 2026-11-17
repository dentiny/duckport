//! Catalog fixtures and Arrow batch helpers for Iceberg sink tests.
use crate::storage;
use arrow_array::{ArrayRef, Int32Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use open_arrow_sink_iceberg::{
    OpenDalStorageFactory,
    iceberg::{
        Catalog, CatalogBuilder, NamespaceIdent, TableCreation, TableIdent,
        memory::MemoryCatalogBuilder,
        spec::{NestedField, PrimitiveType, Schema as IcebergSchema, Type},
    },
};
use std::{collections::HashMap, sync::Arc};

pub(super) async fn table() -> (tempfile::TempDir, Arc<dyn Catalog>, TableIdent) {
    let temp = tempfile::tempdir().unwrap();
    let root = url::Url::from_directory_path(temp.path()).unwrap();
    let factory = OpenDalStorageFactory::new(
        storage::sink_storage_from_root(root.clone(), &Default::default()).unwrap(),
    );
    let catalog: Arc<dyn Catalog> = Arc::new(
        MemoryCatalogBuilder::default()
            .with_storage_factory(Arc::new(factory))
            .load(
                "test",
                HashMap::from([("warehouse".into(), root.to_string())]),
            )
            .await
            .unwrap(),
    );
    let namespace = NamespaceIdent::new("tests".into());
    catalog
        .create_namespace(&namespace, HashMap::new())
        .await
        .unwrap();
    catalog
        .create_table(
            &namespace,
            TableCreation::builder()
                .name("numbers".into())
                .schema(
                    IcebergSchema::builder()
                        .with_fields(vec![Arc::new(NestedField::required(
                            1,
                            "id",
                            Type::Primitive(PrimitiveType::Int),
                        ))])
                        .build()
                        .unwrap(),
                )
                .build(),
        )
        .await
        .unwrap();
    (temp, catalog, TableIdent::new(namespace, "numbers".into()))
}

pub(super) fn batch(values: Vec<Option<i32>>) -> RecordBatch {
    // DuckDB fields are nullable and have no PARQUET:field_id metadata.
    RecordBatch::try_new(
        Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, true)])),
        vec![Arc::new(Int32Array::from(values)) as ArrayRef],
    )
    .unwrap()
}
