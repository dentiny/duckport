use super::{
    open_catalog,
    test_utils::{batch, table},
};

#[tokio::test]
async fn catalog_schema_is_applied_and_required_nulls_abort() {
    let (_temp, catalog, id) = table().await;
    let input = batch(vec![Some(1)]);
    let mut writer = open_catalog(
        catalog.clone(),
        id.clone(),
        input.schema(),
        open_arrow_sink_core::WriteOptions::default(),
        /*writer_count=*/ 1,
        Default::default(),
    )
    .await
    .unwrap();
    let schema = writer.output_schema.as_ref().unwrap();
    assert!(!schema.field(0).is_nullable());
    assert_eq!(
        schema
            .field(0)
            .metadata()
            .get("PARQUET:field_id")
            .map(String::as_str),
        Some("1")
    );
    writer.write(input).await.unwrap();
    assert!(writer.write(batch(vec![None])).await.is_err());
    assert!(writer.commit().await.is_err());
    assert!(
        catalog
            .load_table(&id)
            .await
            .unwrap()
            .metadata()
            .current_snapshot_id()
            .is_none()
    );
}

#[test]
fn ffi_failure_clears_output() {
    use super::ffi::duckport_iceberg_open;
    use crate::{error::ffi_result, options::DuckportStorageOptions};
    use std::ptr;

    let storage = DuckportStorageOptions::default();
    let mut output = ptr::dangling_mut();
    let result = unsafe {
        ffi_result(duckport_iceberg_open(
            c"tests.numbers".as_ptr(),
            ptr::null(),
            c"append".as_ptr(),
            0,
            c"http://127.0.0.1:1".as_ptr(),
            c"".as_ptr(),
            c"/tmp".as_ptr(),
            &crate::ffi::sink::DuckportWriteOptions::default(),
            &storage,
            &mut output,
        ))
    };
    assert!(result.is_err());
    assert!(output.is_null());
}
