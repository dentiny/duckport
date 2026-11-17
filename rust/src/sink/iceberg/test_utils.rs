//! Catalog fixtures and Arrow batch helpers for Iceberg sink tests.
use crate::storage;
use arrow_array::{ArrayRef, Int32Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use open_arrow_sink_iceberg::{
    OpenDalStorageFactory, Storage,
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
        Storage::new(
            storage::create_local_directory_operator(temp.path()).unwrap(),
            root.clone(),
        )
        .unwrap(),
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

pub(super) async fn rows(catalog: &Arc<dyn Catalog>, id: &TableIdent) -> Vec<i32> {
    let table = catalog.load_table(id).await.unwrap();
    let batches: Vec<RecordBatch> = table
        .scan()
        .build()
        .unwrap()
        .to_arrow()
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    let mut values = batches
        .iter()
        .flat_map(|b| {
            b.column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap()
                .values()
                .to_vec()
        })
        .collect::<Vec<_>>();
    values.sort_unstable();
    values
}
