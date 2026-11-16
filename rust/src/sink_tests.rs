use crate::{sink::DuckportSink, sink_ffi::*};
use arrow_array::{
    Array, ArrayRef, Int64Array, LargeBinaryArray, RecordBatch, StringArray, StructArray,
};
use arrow_schema::ffi::FFI_ArrowSchema;
use open_arrow_sink_lance::{LanceDatasetSink, Url};
use std::{collections::HashMap, ffi::CString, ptr, sync::Arc};

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
fn open(
    path: &std::path::Path,
    batch: &RecordBatch,
    mode: &str,
    columns: &[&str],
) -> crate::error::Result<DuckportSink> {
    DuckportSink::new(
        path.to_str().unwrap(),
        batch.schema(),
        mode,
        columns.iter().map(|s| s.to_string()).collect(),
        HashMap::new(),
    )
}
fn read(path: &std::path::Path) -> RecordBatch {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let dataset = LanceDatasetSink::default()
            .dataset_builder(Url::from_directory_path(path).unwrap())
            .unwrap()
            .load()
            .await
            .unwrap();
        dataset
            .scan()
            .blob_handling(lance::datatypes::BlobHandling::AllBinary)
            .try_into_batch()
            .await
            .unwrap()
    })
}
#[test]
fn create_append_overwrite_and_empty_commit() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("dataset");
    let batch = input();
    let mut writer = open(&path, &batch, "create", &[]).unwrap();
    writer.write(batch.clone()).unwrap();
    writer.commit().unwrap();
    assert!(writer.commit().is_err());
    assert_eq!(read(&path).num_rows(), 2);
    assert!(open(&path, &batch, "create", &[]).is_err());
    let mut writer = open(&path, &batch, "append", &[]).unwrap();
    writer.write(batch.clone()).unwrap();
    writer.commit().unwrap();
    assert_eq!(read(&path).num_rows(), 4);
    let mut writer = open(&path, &batch, "overwrite", &[]).unwrap();
    writer.write(batch.clone()).unwrap();
    writer.commit().unwrap();
    let result = read(&path);
    assert_eq!(result.num_rows(), 2);
    assert_eq!(result.schema(), batch.schema());
    assert_eq!(result.column(1).null_count(), 1);
    let mut writer = open(&path, &batch, "overwrite", &[]).unwrap();
    writer.commit().unwrap();
    assert_eq!(read(&path).num_rows(), 0);
}
#[test]
fn download_file_urls_preserves_bytes_nulls_and_other_strings() {
    let temp = tempfile::tempdir().unwrap();
    let payload = b"\x00\xffbinary payload";
    let source = temp.path().join("blob with spaces.bin");
    std::fs::write(&source, payload).unwrap();
    let url = Url::from_file_path(&source).unwrap();
    let batch = RecordBatch::try_from_iter([
        (
            "blob",
            Arc::new(StringArray::from(vec![Some(url.as_str()), None])) as ArrayRef,
        ),
        (
            "text",
            Arc::new(StringArray::from(vec!["keep", "me"])) as ArrayRef,
        ),
    ])
    .unwrap();
    let path = temp.path().join("dataset");
    let mut writer = open(&path, &batch, "create", &["blob"]).unwrap();
    writer.write(batch).unwrap();
    writer.commit().unwrap();
    // Read after deleting the source proves ingestion, rather than storing a reference.
    std::fs::remove_file(source).unwrap();
    let result = read(&path);
    let blobs = result
        .column(0)
        .as_any()
        .downcast_ref::<LargeBinaryArray>()
        .unwrap();
    assert_eq!(blobs.value(0), payload);
    assert!(blobs.is_null(1));
    assert_eq!(
        result
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(0),
        "keep"
    );
}
#[test]
fn failures_and_drop_do_not_publish() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("dataset");
    let batch = input();
    assert!(open(&path, &batch, "create", &["missing"]).is_err());
    assert!(open(&path, &batch, "create", &["id"]).is_err());
    assert!(open(&path, &batch, "invalid", &[]).is_err());
    assert!(open(&path, &batch, "append", &[]).is_err());
    let mut writer = open(&path, &batch, "create", &[]).unwrap();
    writer.write(batch).unwrap();
    drop(writer);
    assert!(!path.join("_latest.manifest").exists());
    assert!(!path.join("_versions/1.manifest").exists());
    let url = Url::from_file_path(temp.path().join("missing.bin")).unwrap();
    let batch = RecordBatch::try_from_iter([(
        "blob",
        Arc::new(StringArray::from(vec![url.as_str()])) as ArrayRef,
    )])
    .unwrap();
    let failed = temp.path().join("failed");
    let mut writer = open(&failed, &batch, "create", &["blob"]).unwrap();
    assert!(writer.write(batch).is_err());
    assert!(writer.commit().is_err());
    assert!(!failed.join("_versions/1.manifest").exists());
}
#[test]
fn ffi_consumes_arrays_and_commits() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ffi");
    let batch = input();
    let schema = FFI_ArrowSchema::try_from(batch.schema().as_ref()).unwrap();
    let path_c = CString::new(path.to_str().unwrap()).unwrap();
    let mode = CString::new("create").unwrap();
    let config = crate::options::DuckportStorageOptions {
        endpoint: ptr::null(),
        region: ptr::null(),
        access_key_id: ptr::null(),
        secret_access_key: ptr::null(),
        session_token: ptr::null(),
        token: ptr::null(),
        virtual_host_style: 0,
        allow_anonymous: 0,
    };
    let mut writer = ptr::null_mut();
    unsafe {
        let error = duckport_lance_open(
            path_c.as_ptr(),
            &schema,
            mode.as_ptr(),
            ptr::null(),
            0,
            &config,
            &mut writer,
        );
        assert!(error.is_null());
        let data = StructArray::from(batch).to_data();
        let (mut array, _) = arrow_array::ffi::to_ffi(&data).unwrap();
        assert!(duckport_sink_write(writer, &mut array).is_null());
        assert!(array.is_released());
        assert!(duckport_sink_commit(writer).is_null());
        duckport_sink_destroy(writer);
    }
    assert_eq!(read(&path).num_rows(), 2);
}

#[test]
fn http_urls_with_query_are_downloaded_and_errors_are_terminal() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = stop.clone();
    let server = std::thread::spawn(move || {
        let mut handlers = Vec::new();
        while !server_stop.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(error) => panic!("{error}"),
            };
            handlers.push(std::thread::spawn(move || {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let count = match stream.read(&mut buffer) {
                        Ok(count) => count,
                        Err(_) => return,
                    };
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                }
                let request = String::from_utf8_lossy(&request);
                if !request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .contains(" /payload?token=test ")
                {
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    return;
                }
                let payload = b"downloaded\x00\xff";
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                if stream.write_all(header.as_bytes()).is_err() {
                    return;
                }
                if !request.starts_with("HEAD ") {
                    let _ = stream.write_all(payload);
                }
            }));
        }
        for handler in handlers {
            handler.join().unwrap();
        }
    });
    // Always stop the server, including when an assertion fails.
    struct Server(Arc<AtomicBool>, Option<std::thread::JoinHandle<()>>);
    impl Drop for Server {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
            let _ = self.1.take().unwrap().join();
        }
    }
    let server = Server(stop, Some(server));
    let temp = tempfile::tempdir().unwrap();
    let url = format!("http://{address}/payload?token=test");
    let batch = RecordBatch::try_from_iter([(
        "blob",
        Arc::new(StringArray::from(vec![Some(url.as_str()), None])) as ArrayRef,
    )])
    .unwrap();
    let path = temp.path().join("http");
    let mut writer = open(&path, &batch, "create", &["blob"]).unwrap();
    writer.write(batch).unwrap();
    writer.commit().unwrap();
    let missing_url = format!("http://{address}/missing");
    let batch = RecordBatch::try_from_iter([(
        "blob",
        Arc::new(StringArray::from(vec![missing_url])) as ArrayRef,
    )])
    .unwrap();
    let mut writer = open(&temp.path().join("failed"), &batch, "create", &["blob"]).unwrap();
    assert!(writer.write(batch).is_err());
    assert!(writer.commit().is_err());
    drop(server);
    let result = read(&path);
    let blobs = result
        .column(0)
        .as_any()
        .downcast_ref::<LargeBinaryArray>()
        .unwrap();
    assert_eq!(blobs.value(0), b"downloaded\x00\xff");
    assert!(blobs.is_null(1));
}
