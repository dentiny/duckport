use crate::error::Result;
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use open_arrow_sink_core::{Sink, SinkExt, WriteMode, WriteOptions, Writer};
use open_arrow_sink_lance::{LanceDatasetSink, Storage, Url};
use open_arrow_sink_layer::DownloadBlobLayer;
use std::collections::{HashMap, HashSet};
use tokio::runtime::Runtime;

pub struct DuckportSink {
    // Drop the writer inside its runtime before dropping the runtime itself.
    writer: Option<Box<dyn Writer>>,
    runtime: Runtime,
    pub(crate) schema: SchemaRef,
}
impl DuckportSink {
    pub(crate) fn new(
        path: &str,
        schema: SchemaRef,
        mode: &str,
        columns: HashSet<String>,
        config: HashMap<String, String>,
    ) -> Result<Self> {
        let mode = match mode {
            "create" => WriteMode::Create,
            "append" => WriteMode::Append,
            "overwrite" => WriteMode::Overwrite,
            _ => return Err("Lance mode must be create, append, or overwrite".into()),
        };
        let url = if path.contains("://") {
            Url::parse(path)?
        } else {
            Url::from_directory_path(std::path::absolute(path)?)
                .map_err(|_| "invalid dataset path")?
        };
        if !matches!(url.scheme(), "file" | "s3") {
            return Err("Lance destination must be a local path, file:// URI, or s3:// URI".into());
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let writer: Result<Box<dyn Writer>> = runtime.block_on(async {
            let sink = if config.is_empty() {
                LanceDatasetSink::default()
            } else {
                let mut config = config;
                if config.contains_key("root") {
                    return Err("Lance destination already determines storage root".into());
                }
                let mut root = url.clone();
                root.set_path("/");
                opendal::install_default();
                let operator = if root.scheme() == "file" {
                    config.insert("root".into(), "/".into());
                    opendal::Operator::via_iter("fs", config)?
                } else {
                    opendal::Operator::from_uri(opendal::OperatorUri::new(root.as_str(), config)?)?
                };
                LanceDatasetSink::new(Storage::new(operator, root)?)
            };
            let sink = sink.layer(DownloadBlobLayer::new(columns));
            sink.open(url, schema.clone(), WriteOptions::default().with_mode(mode))
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        });
        let writer = writer?;
        Ok(Self {
            writer: Some(writer),
            runtime,
            schema,
        })
    }
    pub(crate) fn write(&mut self, batch: RecordBatch) -> Result<()> {
        let writer = self.writer.as_mut().ok_or("Lance writer is closed")?;
        if let Err(error) = self.runtime.block_on(writer.write(batch)) {
            if let Some(writer) = self.writer.take() {
                let _ = self.runtime.block_on(writer.abort());
            }
            return Err(error.into());
        }
        Ok(())
    }
    pub(crate) fn commit(&mut self) -> Result<()> {
        let writer = self.writer.take().ok_or("Lance writer is closed")?;
        self.runtime.block_on(writer.commit())?;
        Ok(())
    }
}
impl Drop for DuckportSink {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            // Dropping an unfinished SQL COPY must never publish a manifest.
            let _guard = self.runtime.enter();
            drop(writer);
        }
    }
}
