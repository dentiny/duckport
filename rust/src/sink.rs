use crate::{error::Result, ffi::RUNTIME, options::StorageConfig, storage};
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{Sink, SinkExt, WriteMode, WriteOptions, Writer};
use open_arrow_sink_lance::{LanceDatasetSink, Storage, Url};
use open_arrow_sink_layer::DownloadBlobLayer;
use std::collections::HashSet;

pub struct DuckportSink {
    writer: Option<Box<dyn Writer>>,
    pub(crate) schema: SchemaRef,
}
impl DuckportSink {
    pub(crate) fn new(
        path: &str,
        schema: SchemaRef,
        mode: &str,
        columns: HashSet<String>,
        config: &StorageConfig,
    ) -> Result<Self> {
        let mode = match mode {
            "create" => WriteMode::Create,
            "append" => WriteMode::Append,
            "overwrite" => WriteMode::Overwrite,
            _ => return Err("Lance mode must be create, append, or overwrite".into()),
        };
        let mut url = if path.contains("://") {
            Url::parse(path)?
        } else {
            Url::from_directory_path(storage::local_path(path)?)
                .map_err(|_| "invalid dataset path")?
        };
        crate::validation::validate_uri(&url)?;
        if !matches!(url.scheme(), "file" | "s3") {
            return Err("Lance destination must be a local path, file:// URI, or s3:// URI".into());
        }
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        let writer: Result<Box<dyn Writer>> = RUNTIME.block_on(async {
            let (operator, _) = storage::operator(url.as_str(), config)?;
            // The shared resolver roots local operators at the dataset's parent
            // and S3 operators at the bucket root. Match that URL namespace.
            let mut root = url.join("..")?;
            if url.scheme() == "s3" {
                root.set_path("/");
            }
            let sink = LanceDatasetSink::new(Storage::new(operator, root)?)
                .layer(DownloadBlobLayer::new(columns));
            Ok(sink
                .open(url, schema.clone(), WriteOptions::default().with_mode(mode))
                .await?)
        });
        Ok(Self {
            writer: Some(writer?),
            schema,
        })
    }
    pub(crate) fn write(&mut self, batch: RecordBatch) -> Result<()> {
        let writer = self.writer.as_mut().ok_or("Lance writer is closed")?;
        if let Err(error) = RUNTIME.block_on(writer.write(batch)) {
            if let Some(writer) = self.writer.take() {
                let _ = RUNTIME.block_on(writer.abort());
            }
            return Err(error.into());
        }
        Ok(())
    }
    pub(crate) fn commit(&mut self) -> Result<()> {
        let writer = self.writer.take().ok_or("Lance writer is closed")?;
        RUNTIME.block_on(writer.commit())?;
        Ok(())
    }
}
impl Drop for DuckportSink {
    fn drop(&mut self) {
        // Dropping an unfinished SQL COPY must never publish a manifest.
        let _guard = RUNTIME.enter();
        self.writer.take();
    }
}
