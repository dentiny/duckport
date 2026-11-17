use super::ffi::duckport_lance_open;
use crate::{
    error::{Result, ffi_result},
    ffi::sink::*,
    options::{DuckportStorageOptions, StorageConfig},
    sink::DuckportSink,
};
use arrow_array::{
    Array, ArrayRef, Int64Array, RecordBatch, StringArray, StructArray, cast::AsArray,
    types::Int64Type,
};
use arrow_schema::ffi::FFI_ArrowSchema;
use open_arrow_sink_core::WriteOptions;
use open_arrow_sink_lance::{LanceDatasetSink, Url};
use std::{ffi::CString, path::Path, ptr, sync::Arc};

async fn tempdir() -> tempfile::TempDir {
    tokio::task::spawn_blocking(tempfile::tempdir)
        .await
        .unwrap()
        .unwrap()
}

fn input() -> RecordBatch {
    RecordBatch::try_from_iter([
        ("id", Arc::new(Int64Array::from(vec![1, 2])) as ArrayRef),
        (
            "text",
            Arc::new(StringArray::from(vec![
                Some("https://example.com/keep-as-text"),
                None,
            ])) as ArrayRef,
        ),
    ])
    .unwrap()
}
fn blob_input(url: &str) -> RecordBatch {
    RecordBatch::try_from_iter([
        (
            "blob",
            Arc::new(StringArray::from(vec![Some(url), None])) as ArrayRef,
        ),
        (
            "text",
            Arc::new(StringArray::from(vec!["keep", "me"])) as ArrayRef,
        ),
    ])
    .unwrap()
}
async fn open(
    path: &Path,
    batch: &RecordBatch,
    mode: &str,
    columns: &[&str],
) -> Result<DuckportSink> {
    super::open(
        path.to_str().unwrap(),
        batch.schema(),
        mode,
        columns.iter().map(|s| s.to_string()).collect(),
        &StorageConfig::default(),
        WriteOptions::default(),
        /*writer_count=*/1,
        Vec::new(),
    )
    .await
}
async fn read(path: &Path) -> Result<RecordBatch> {
    let dataset = LanceDatasetSink::default()
        .dataset_builder(Url::from_directory_path(path).unwrap())?
        .load()
        .await?;
    Ok(dataset
        .scan()
        .blob_handling(lance::datatypes::BlobHandling::AllBinary)
        .try_into_batch()
        .await?)
}
fn ids(batch: &RecordBatch) -> Vec<i64> {
    let mut ids = batch
        .column(0)
        .as_primitive::<Int64Type>()
        .values()
        .to_vec();
    ids.sort_unstable();
    ids
}
#[tokio::test]
async fn create_append_overwrite_and_empty_commit() {
    let temp = tempdir().await;
    let path = temp.path().join("dataset");
    let batch = input();
    for (mode, expected) in [
        ("create", vec![1, 2]),
        ("append", vec![1, 1, 2, 2]),
        ("overwrite", vec![1, 2]),
    ] {
        let mut writer = open(&path, &batch, mode, &[]).await.unwrap();
        writer.write(batch.clone()).await.unwrap();
        writer.commit().await.unwrap();
        let result = read(&path).await.unwrap();
        assert_eq!(ids(&result), expected);
        assert_eq!(result.schema(), batch.schema());
        let strings = result.column(1).as_string::<i32>();
        assert_eq!(strings.null_count(), expected.len() / 2);
        assert!(
            strings
                .iter()
                .flatten()
                .all(|s| s == "https://example.com/keep-as-text")
        );
    }
    assert!(open(&path, &batch, "create", &[]).await.is_err());
    let mut writer = open(&path, &batch, "overwrite", &[]).await.unwrap();
    writer.commit().await.unwrap();
    assert_eq!(read(&path).await.unwrap().num_rows(), 0);
}
#[tokio::test]
async fn invalid_inputs_and_abandoned_create_do_not_publish() {
    let temp = tempdir().await;
    let path = temp.path().join("dataset");
    let batch = input();
    assert!(open(&path, &batch, "create", &["missing"]).await.is_err());
    assert!(open(&path, &batch, "create", &["id"]).await.is_err());
    assert!(open(&path, &batch, "invalid", &[]).await.is_err());
    assert!(open(&path, &batch, "append", &[]).await.is_err());
    let mut writer = open(&path, &batch, "create", &[]).await.unwrap();
    writer.write(batch).await.unwrap();
    drop(writer);
    assert!(read(&path).await.is_err());
    let url = Url::from_file_path(temp.path().join("missing.bin")).unwrap();
    let batch = blob_input(url.as_str());
    let failed = temp.path().join("failed");
    let mut writer = open(&failed, &batch, "create", &["blob"]).await.unwrap();
    assert!(writer.write(batch).await.is_err());
    assert!(writer.commit().await.is_err());
    assert!(read(&failed).await.is_err());
}
#[tokio::test]
async fn aborted_writes_preserve_committed_rows() {
    let temp = tempdir().await;
    let path = temp.path().join("dataset");
    let batch = input();
    let mut writer = open(&path, &batch, "create", &[]).await.unwrap();
    writer.write(batch.clone()).await.unwrap();
    writer.commit().await.unwrap();
    let pending = RecordBatch::try_new(
        batch.schema(),
        vec![
            Arc::new(Int64Array::from(vec![3, 4])) as ArrayRef,
            batch.column(1).clone(),
        ],
    )
    .unwrap();
    for mode in ["append", "overwrite"] {
        let mut writer = open(&path, &batch, mode, &[]).await.unwrap();
        writer.write(pending.clone()).await.unwrap();
        drop(writer);
        assert_eq!(ids(&read(&path).await.unwrap()), vec![1, 2]);
        let mut writer = open(&path, &batch, mode, &[]).await.unwrap();
        writer.write(pending.clone()).await.unwrap();
        assert!(writer.write(batch.project(&[0]).unwrap()).await.is_err());
        assert!(writer.commit().await.is_err());
        assert_eq!(ids(&read(&path).await.unwrap()), vec![1, 2]);
    }
}
#[tokio::test]
async fn ffi_ingests_urls_and_preserves_ownership_and_metadata() {
    let temp = tempdir().await;
    let path = temp.path().join("nested data/dataset with spaces");
    let source = temp.path().join("blob with spaces.bin");
    let payload = b"\x00\xffbinary payload";
    tokio::fs::write(&source, payload).await.unwrap();
    let batch = blob_input(Url::from_file_path(&source).unwrap().as_str());
    let schema = batch
        .schema()
        .as_ref()
        .clone()
        .with_metadata([("owner".into(), "duckport".into())].into());
    let batch = batch.with_schema(Arc::new(schema)).unwrap();
    let destination = path.clone();
    tokio::task::spawn_blocking(move || {
        let mut writer = ptr::null_mut();
        unsafe {
            let schema = FFI_ArrowSchema::try_from(batch.schema().as_ref()).unwrap();
            let path_c = CString::new(Url::from_file_path(&destination).unwrap().as_str()).unwrap();
            let mode = CString::new("create").unwrap();
            let column = CString::new("blob").unwrap();
            ffi_result(duckport_lance_open(
                path_c.as_ptr(),
                &schema,
                mode.as_ptr(),
                [column.as_ptr()].as_ptr(),
                1,
                &DuckportWriteOptions::default(),
                &DuckportStorageOptions::default(),
                &mut writer,
            ))
            .unwrap();
        }
        // The schema, option strings and pointer array above have all been dropped.
        unsafe {
            let (mut array, _) =
                arrow_array::ffi::to_ffi(&StructArray::from(batch).to_data()).unwrap();
            ffi_result(duckport_sink_write(writer, &mut array)).unwrap();
            assert!(array.is_released());
            ffi_result(duckport_sink_commit(writer)).unwrap();
            assert!(ffi_result(duckport_sink_commit(writer)).is_err());
            duckport_sink_destroy(writer);
        }
    })
    .await
    .unwrap();
    tokio::fs::remove_file(source).await.unwrap();
    let result = read(&path).await.unwrap();
    assert_eq!(
        result.schema().metadata().get("owner").map(String::as_str),
        Some("duckport")
    );
    let labels = result.column(1).as_string::<i32>();
    let blobs = result.column(0).as_binary::<i64>();
    let mut rows = labels.iter().zip(blobs.iter()).collect::<Vec<_>>();
    rows.sort();
    assert_eq!(
        rows,
        [(Some("keep"), Some(payload.as_slice())), (Some("me"), None)]
    );
}
#[tokio::test]
async fn ffi_rejects_null_handles_and_consumes_rejected_arrays() {
    tokio::task::spawn_blocking(|| {
        let batch = input();
        let schema = FFI_ArrowSchema::try_from(batch.schema().as_ref()).unwrap();
        let path = CString::new("unused").unwrap();
        let mode = CString::new("create").unwrap();
        let options = DuckportStorageOptions::default();
        let mut output = ptr::dangling_mut();
        unsafe {
            assert!(
                ffi_result(duckport_lance_open(
                    path.as_ptr(),
                    ptr::null(),
                    mode.as_ptr(),
                    ptr::null(),
                    0,
                    &DuckportWriteOptions::default(),
                    &options,
                    &mut output
                ))
                .is_err()
            );
            assert!(output.is_null());
            assert!(
                ffi_result(duckport_lance_open(
                    path.as_ptr(),
                    &schema,
                    mode.as_ptr(),
                    ptr::null(),
                    1,
                    &DuckportWriteOptions::default(),
                    &options,
                    &mut output
                ))
                .is_err()
            );
            assert!(output.is_null());
            let (mut array, _) =
                arrow_array::ffi::to_ffi(&StructArray::from(batch).to_data()).unwrap();
            assert!(ffi_result(duckport_sink_write(ptr::null_mut(), &mut array)).is_err());
            assert!(array.is_released());
            assert!(ffi_result(duckport_sink_commit(ptr::null_mut())).is_err());
            duckport_sink_destroy(ptr::null_mut());
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn atif_nested_media_is_ingested_on_write() {
    let temp = tempdir().await;
    let path = temp.path().join("trajectory.json");
    let image = temp.path().join("image.bin");
    tokio::fs::write(&image, b"image payload").await.unwrap();
    tokio::fs::write(&path, r#"{
        "schema_version":"ATIF-v1.8", "trajectory_id":"media",
        "agent":{"name":"test","version":"1"},
        "steps":[{"step_id":1,"source":"user","message":[
            {"type":"text","text":"hello"},
            {"type":"image","source":{"media_type":"image/png","path":"image.bin"}},
            {"type":"audio","source":{"media_type":"audio/wav","path":"data:audio/wav;base64,YXVkaW8=","duration_sec":1.25}}
        ]}]
    }"#).await.unwrap();
    let (operator, file) =
        crate::storage::operator(path.to_str().unwrap(), &Default::default()).unwrap();
    let source = open_arrow_access::open_file_source(
        operator,
        open_arrow_access::ReadRequest::new(file),
        open_arrow_access::file_format::atif::AtifFormat::Auto,
        open_arrow_access::OuterCompression::Auto,
    )
    .await
    .unwrap();
    let source = crate::source::DuckportSource::from(source);
    let schema = crate::atif::source_schema(&source.schema(), path.to_str().unwrap()).unwrap();
    let source = source.with_schema(schema.clone());
    let mut reader = source.reader().await.unwrap();
    let input = reader.next().await.unwrap().unwrap();
    let destination = temp.path().join("dataset");
    let mut writer = super::open(
        destination.to_str().unwrap(),
        schema,
        "create",
        Default::default(),
        &Default::default(),
        WriteOptions::default(),
        /*writer_count=*/1,
        Vec::new(),
    )
    .await
    .unwrap();
    writer.write(input).await.unwrap();
    writer.commit().await.unwrap();
    tokio::fs::remove_file(image).await.unwrap();
    let actual = read(&destination).await.unwrap();
    let steps = actual.column_by_name("steps").unwrap().as_list::<i32>();
    let steps = steps.values().as_struct();
    let messages = steps.column_by_name("message").unwrap().as_list::<i32>();
    let content = messages.values().as_struct();
    assert_eq!(
        content
            .column_by_name("text")
            .unwrap()
            .as_binary::<i64>()
            .value(0),
        b"hello"
    );
    let media = content.column_by_name("source").unwrap().as_struct();
    assert!(media.is_null(0));
    let bytes = media.column_by_name("path").unwrap().as_binary::<i64>();
    assert_eq!(bytes.value(1), b"image payload");
    assert_eq!(bytes.value(2), b"audio");
    assert_eq!(
        media
            .column_by_name("media_type")
            .unwrap()
            .as_string::<i32>()
            .value(1),
        "image/png"
    );
    assert_eq!(
        media
            .column_by_name("duration_sec")
            .unwrap()
            .as_primitive::<arrow_array::types::Float64Type>()
            .value(2),
        1.25
    );
}
