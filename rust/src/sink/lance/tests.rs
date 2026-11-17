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
                &DuckportWriteOptions {
                    blob_inline_size_threshold: 0,
                    blob_dedicated_size_threshold: 1,
                    target_file_size: 1,
                },
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
    let dataset = LanceDatasetSink::default()
        .dataset_builder(Url::from_directory_path(&path).unwrap())
        .unwrap()
        .load()
        .await
        .unwrap();
    let field = dataset.schema().field("blob").unwrap();
    assert_eq!(
        field
            .metadata
            .get("lance-encoding:blob-inline-size-threshold")
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(
        field
            .metadata
            .get("lance-encoding:blob-dedicated-size-threshold")
            .map(String::as_str),
        Some("1")
    );
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
async fn configured_thresholds_control_blob_layout() {
    use lance::datatypes::BlobKind;
    let temp = tempdir().await;
    let options = DuckportWriteOptions {
        blob_inline_size_threshold: 1,
        blob_dedicated_size_threshold: 2,
        target_file_size: -1,
    }
    .read(0)
    .unwrap();
    for (payload, kind) in [
        ("x", BlobKind::Inline),
        ("xx", BlobKind::Managed),
        ("xxx", BlobKind::Managed),
    ] {
        let source = temp.path().join(format!("source-{}.bin", payload.len()));
        tokio::fs::write(&source, payload).await.unwrap();
        let batch = blob_input(Url::from_file_path(&source).unwrap().as_str());
        let path = temp.path().join(format!("dataset-{}", payload.len()));
        let mut writer = super::open(
            path.to_str().unwrap(),
            batch.schema(),
            "create",
            ["blob".to_string()].into(),
            &StorageConfig::default(),
            options.clone(),
        )
        .await
        .unwrap();
        writer.write(batch.clone()).await.unwrap();
        writer.write(batch).await.unwrap();
        writer.commit().await.unwrap();
        tokio::fs::remove_file(source).await.unwrap();
        let dataset = LanceDatasetSink::default()
            .dataset_builder(Url::from_directory_path(&path).unwrap())
            .unwrap()
            .load()
            .await
            .unwrap();
        assert_eq!(dataset.count_rows(None).await.unwrap(), 4);
        let blobs = Arc::new(dataset)
            .take_blobs_by_indices(&[0, 1, 2, 3], "blob")
            .await
            .unwrap();
        let present = blobs.iter().flatten().collect::<Vec<_>>();
        assert_eq!(present.len(), 2);
        if payload.len() == 2 {
            assert_eq!(present[0].data_path(), present[1].data_path());
        } else if payload.len() == 3 {
            assert_ne!(present[0].data_path(), present[1].data_path());
        }
        for blob in present {
            assert_eq!(blob.kind(), kind);
            assert_eq!(blob.read(0).await.unwrap().as_ref(), payload.as_bytes());
        }
    }
}

#[test]
fn size_options_validate_boundaries_and_preserve_defaults() {
    let defaults = DuckportWriteOptions::default();
    let options = defaults.read(0).unwrap();
    assert!(options.blob_handle.is_none());
    assert!(options.target_file_size.is_none());
    // Selecting a URL column must request blob capability even without thresholds.
    assert!(defaults.read(1).unwrap().blob_handle.is_some());
    assert!(
        DuckportWriteOptions {
            blob_inline_size_threshold: 0,
            ..defaults
        }
        .read(0)
        .is_ok()
    );
    for invalid in [
        DuckportWriteOptions {
            blob_inline_size_threshold: -2,
            ..defaults
        },
        DuckportWriteOptions {
            blob_dedicated_size_threshold: 0,
            ..defaults
        },
        DuckportWriteOptions {
            target_file_size: 0,
            ..defaults
        },
        DuckportWriteOptions {
            target_file_size: -2,
            ..defaults
        },
    ] {
        assert!(invalid.read(0).is_err());
    }
}

#[tokio::test]
async fn target_file_size_rotates_files_without_losing_rows() {
    let temp = tempdir().await;
    let batch = RecordBatch::try_from_iter([
        (
            "id",
            Arc::new(Int64Array::from_iter_values(0..10240)) as ArrayRef,
        ),
        (
            "text",
            Arc::new(StringArray::from_iter_values(
                (0..10240).map(|id| format!("{id}:{}", "payload ".repeat(256))),
            )) as ArrayRef,
        ),
        (
            "raw",
            Arc::new(
                arrow_array::FixedSizeBinaryArray::try_from_iter((0..10240).map(|id| {
                    (0..1024)
                        .map(|offset| ((id + offset) % 251) as u8)
                        .collect::<Vec<_>>()
                }))
                .unwrap(),
            ) as ArrayRef,
        ),
    ])
    .unwrap();
    for (label, target_file_size) in [("default", -1), ("small", 1)] {
        let path = temp.path().join(label);
        let options = DuckportWriteOptions {
            target_file_size,
            ..Default::default()
        }
        .read(0)
        .unwrap();
        let mut writer = super::open(
            path.to_str().unwrap(),
            batch.schema(),
            "create",
            Default::default(),
            &StorageConfig::default(),
            options,
        )
        .await
        .unwrap();
        writer.write(batch.clone()).await.unwrap();
        writer.write(batch.clone()).await.unwrap();
        writer.commit().await.unwrap();
        let dataset = LanceDatasetSink::default()
            .dataset_builder(Url::from_directory_path(&path).unwrap())
            .unwrap()
            .load()
            .await
            .unwrap();
        assert_eq!(dataset.count_rows(None).await.unwrap(), 20480);
        if target_file_size == -1 {
            assert_eq!(dataset.get_fragments().len(), 1);
        } else {
            assert!(dataset.get_fragments().len() > 1);
        }
        let actual = dataset.scan().try_into_batch().await.unwrap();
        let mut expected = (0..10240).chain(0..10240).collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(ids(&actual), expected);
    }
}
