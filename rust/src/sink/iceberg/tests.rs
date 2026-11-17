use super::open_catalog;
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

async fn table() -> (tempfile::TempDir, Arc<dyn Catalog>, TableIdent) {
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
fn batch(values: Vec<Option<i32>>) -> RecordBatch {
    // DuckDB fields are nullable and have no PARQUET:field_id metadata.
    RecordBatch::try_new(
        Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, true)])),
        vec![Arc::new(Int32Array::from(values)) as ArrayRef],
    )
    .unwrap()
}
async fn rows(catalog: &Arc<dyn Catalog>, id: &TableIdent) -> Vec<i32> {
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
#[tokio::test]
async fn append_normalizes_field_ids_and_commits_only_on_finalize() {
    let (_temp, catalog, id) = table().await;
    let batch = batch(vec![Some(1), Some(2)]);
    for expected in [vec![1, 2], vec![1, 1, 2, 2]] {
        let mut writer = open_catalog(catalog.clone(), id.clone(), batch.schema())
            .await
            .unwrap();
        writer.write(batch.clone()).await.unwrap();
        writer.commit().await.unwrap();
        assert!(writer.commit().await.is_err());
        assert_eq!(rows(&catalog, &id).await, expected);
    }
    let before = catalog
        .load_table(&id)
        .await
        .unwrap()
        .metadata()
        .current_snapshot_id();
    let mut empty = open_catalog(catalog.clone(), id.clone(), batch.schema())
        .await
        .unwrap();
    empty.commit().await.unwrap();
    assert_eq!(
        catalog
            .load_table(&id)
            .await
            .unwrap()
            .metadata()
            .current_snapshot_id(),
        before
    );
    let mut abandoned = open_catalog(catalog.clone(), id.clone(), batch.schema())
        .await
        .unwrap();
    abandoned.write(batch).await.unwrap();
    drop(abandoned);
    assert_eq!(rows(&catalog, &id).await, vec![1, 1, 2, 2]);
}
#[tokio::test]
async fn invalid_schema_and_required_nulls_do_not_publish() {
    let (_temp, catalog, id) = table().await;
    let batch = batch(vec![Some(1), None]);
    let wrong = Arc::new(Schema::new(vec![Field::new(
        "other",
        DataType::Int32,
        true,
    )]));
    assert!(
        open_catalog(catalog.clone(), id.clone(), wrong)
            .await
            .is_err()
    );
    let mut writer = open_catalog(catalog.clone(), id.clone(), batch.schema())
        .await
        .unwrap();
    writer.write(self::batch(vec![Some(3)])).await.unwrap();
    assert!(writer.write(batch).await.is_err());
    assert!(writer.commit().await.is_err());
    assert!(rows(&catalog, &id).await.is_empty());
    let missing = TableIdent::new(NamespaceIdent::new("tests".into()), "missing".into());
    assert!(
        open_catalog(catalog, missing, self::batch(vec![]).schema())
            .await
            .is_err()
    );
}
#[test]
fn ffi_failure_clears_output_and_validates_pointers() {
    use crate::{
        error::ffi_result, options::DuckportStorageOptions,
        sink::iceberg::ffi::duckport_iceberg_open,
    };
    use arrow_schema::ffi::FFI_ArrowSchema;
    use std::{ffi::CString, ptr};
    let schema = FFI_ArrowSchema::try_from(batch(vec![]).schema().as_ref()).unwrap();
    let identifier = CString::new("invalid").unwrap();
    let uri = CString::new("http://127.0.0.1:1").unwrap();
    let empty = CString::new("").unwrap();
    let root = CString::new("/tmp").unwrap();
    let storage = DuckportStorageOptions::default();
    let mut output = ptr::dangling_mut();
    unsafe {
        assert!(
            ffi_result(duckport_iceberg_open(
                identifier.as_ptr(),
                &schema,
                uri.as_ptr(),
                empty.as_ptr(),
                root.as_ptr(),
                &storage,
                &mut output
            ))
            .is_err()
        );
        assert!(output.is_null());
        assert!(
            ffi_result(duckport_iceberg_open(
                identifier.as_ptr(),
                ptr::null(),
                uri.as_ptr(),
                empty.as_ptr(),
                root.as_ptr(),
                &storage,
                &mut output
            ))
            .is_err()
        );
        assert!(output.is_null());
        assert!(
            ffi_result(duckport_iceberg_open(
                identifier.as_ptr(),
                &schema,
                uri.as_ptr(),
                empty.as_ptr(),
                root.as_ptr(),
                &storage,
                ptr::null_mut()
            ))
            .is_err()
        );
    }
}

#[tokio::test]
async fn nested_columns_receive_catalog_field_ids() {
    use arrow_array::StructArray;
    use open_arrow_sink_iceberg::iceberg::spec::StructType;
    let (_temp, catalog, _) = table().await;
    let namespace = NamespaceIdent::new("tests".into());
    catalog
        .create_table(
            &namespace,
            TableCreation::builder()
                .name("nested".into())
                .schema(
                    IcebergSchema::builder()
                        .with_fields(vec![Arc::new(NestedField::optional(
                            1,
                            "payload",
                            Type::Struct(StructType::new(vec![Arc::new(NestedField::optional(
                                2,
                                "value",
                                Type::Primitive(PrimitiveType::Int),
                            ))])),
                        ))])
                        .build()
                        .unwrap(),
                )
                .build(),
        )
        .await
        .unwrap();
    let values = Arc::new(StructArray::from(vec![(
        Arc::new(Field::new("value", DataType::Int32, true)),
        Arc::new(Int32Array::from(vec![Some(7), None])) as ArrayRef,
    )]));
    let batch = RecordBatch::try_from_iter([("payload", values as ArrayRef)]).unwrap();
    let id = TableIdent::new(namespace, "nested".into());
    let mut writer = open_catalog(catalog.clone(), id.clone(), batch.schema())
        .await
        .unwrap();
    writer.write(batch).await.unwrap();
    writer.commit().await.unwrap();
    let table = catalog.load_table(&id).await.unwrap();
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
    assert_eq!(batches.iter().map(RecordBatch::num_rows).sum::<usize>(), 2);
    let values = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<StructArray>()
        .unwrap()
        .column(0)
        .as_any()
        .downcast_ref::<Int32Array>()
        .unwrap();
    assert_eq!(values, &Int32Array::from(vec![Some(7), None]));
}
