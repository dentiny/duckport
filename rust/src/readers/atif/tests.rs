use super::*;
use crate::{error::Result, options::StorageConfig};
use arrow_array::{Array, RecordBatch, StructArray, cast::AsArray};
use std::path::Path;

async fn read(path: &Path) -> Result<Vec<RecordBatch>> {
    let source = open(
        path.to_str().unwrap(),
        StorageConfig::default(),
        Default::default(),
    )
    .await?;
    let mut reader = source.reader().await?;
    let mut batches = Vec::new();
    while let Some(batch) = reader.next().await? {
        batches.push(batch);
    }
    Ok(batches)
}
fn message(batch: &RecordBatch) -> StructArray {
    let steps = batch["steps"].as_list::<i32>().value(0);
    steps.as_struct()["message"]
        .as_list::<i32>()
        .value(0)
        .as_struct()
        .clone()
}

#[tokio::test]
async fn embeds_local_media_and_preserves_raw_json_and_variants() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../testdata/atif");
    let batches = read(&fixtures.join("trajectory_media.json")).await.unwrap();
    let batch = &batches[0];
    let parts = message(batch);
    let paths = &parts["source"].as_struct()["path"].as_binary::<i64>();
    assert_eq!(
        paths.value(1),
        std::fs::read(fixtures.join("atif-image.png")).unwrap()
    );
    assert_eq!(
        paths.value(2),
        std::fs::read(fixtures.join("atif-audio.wav")).unwrap()
    );
    assert_eq!(
        parts["text"].as_binary::<i64>().value(0),
        "hello 你好🙂".as_bytes()
    );
    assert_eq!(parts["type"].as_string::<i32>().value(2), "audio");
    assert!(parts["source"].is_null(0));
    let steps = batch["steps"].as_list::<i32>().value(0);
    let results = steps.as_struct()["observation"].as_struct()["results"]
        .as_list::<i32>()
        .value(1);
    let content = results.as_struct()["content"].as_list::<i32>().value(0);
    assert_eq!(
        content.as_struct()["source"].as_struct()["path"]
            .as_binary::<i64>()
            .value(1),
        paths.value(1)
    );
    assert_eq!(
        batch["raw_json"].as_string::<i32>().value(0),
        std::fs::read_to_string(fixtures.join("trajectory_media.json")).unwrap()
    );
}

#[tokio::test]
async fn embeds_jsonl_data_uris_and_reports_missing_media() {
    let temp = tempfile::tempdir().unwrap();
    let doc = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../testdata/atif/trajectory_media.json"),
    )
    .unwrap()
    .replace("atif-image.png", "data:image/png;base64,AP8=")
    .replace("atif-audio.wav", "data:audio/wav,%00%ff");
    // Auto framing accepts two JSON objects, including pretty-printed JSONL.
    let path = temp.path().join("trajectories.jsonl");
    std::fs::write(&path, format!("{doc}\n{doc}\n")).unwrap();
    let batches = read(&path).await.unwrap();
    assert_eq!(batches.iter().map(RecordBatch::num_rows).sum::<usize>(), 2);
    assert_eq!(
        message(&batches[0])["source"].as_struct()["path"]
            .as_binary::<i64>()
            .value(1),
        [0, 255]
    );
    std::fs::write(
        &path,
        doc.replace("data:image/png;base64,AP8=", "missing.png"),
    )
    .unwrap();
    assert!(
        read(&path)
            .await
            .unwrap_err()
            .to_string()
            .contains("ATIF media missing.png")
    );
}

#[tokio::test]
async fn gzip_resolves_local_media_relative_to_compressed_document() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../testdata/atif");
    let batches = read(&fixtures.join("trajectory_media.jsonl.gz"))
        .await
        .unwrap();
    assert_eq!(batches.len(), 2);
    for batch in batches {
        assert_eq!(
            message(&batch)["source"].as_struct()["path"]
                .as_binary::<i64>()
                .value(1),
            std::fs::read(fixtures.join("atif-image.png")).unwrap()
        );
    }
}
