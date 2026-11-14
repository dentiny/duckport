use crate::{error::Result, options::StorageConfig, storage};
use arrow_schema::SchemaRef;
use open_arrow_access::{
    Accessor, AccessorExt, ReadOptions, ReadRequest, ReaderFactory, RecordReader,
    decompression::AutoDecompressionLayer, file_format::FileAccessor, storage::OpenDalAccessor,
};
use std::sync::Mutex;

pub(crate) enum OuterCompression {
    None,
    Auto,
}

/// Resolve storage and bind a typed format with an explicit outer-compression policy.
pub(crate) async fn open_file_source<F>(
    locator: &str,
    storage_config: &StorageConfig,
    options: ReadOptions,
    format: F,
    compression: OuterCompression,
) -> Result<DuckportSource>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + 'static,
{
    let (operator, path) = storage::operator(locator, storage_config)?;
    let storage = OpenDalAccessor::new(operator);
    let request = ReadRequest::new(path).with_options(options);
    match compression {
        OuterCompression::None => {
            // Parquet needs the original byte representation and its range access.
            DuckportSource::new(FileAccessor::from_format(storage, format), request).await
        }
        OuterCompression::Auto => {
            let storage = storage.layer(AutoDecompressionLayer::new());
            DuckportSource::new(FileAccessor::from_format(storage, format), request).await
        }
    }
}

pub struct DuckportSource {
    schema: SchemaRef,
    accessor: Box<dyn Accessor<Request = ReadRequest, Reader = dyn RecordReader>>,
    request: ReadRequest,
    bound: Mutex<Option<Box<dyn RecordReader>>>,
}
impl DuckportSource {
    pub async fn new(
        accessor: impl Accessor<Request = ReadRequest, Reader = dyn RecordReader> + 'static,
        request: ReadRequest,
    ) -> Result<Self> {
        let reader = accessor.open(request.clone()).await?;
        let schema = reader.metadata().schema.clone();
        Ok(Self {
            schema,
            accessor: Box::new(accessor),
            request,
            bound: Mutex::new(Some(reader)),
        })
    }
    pub fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    pub async fn reader(&self) -> Result<Box<dyn RecordReader>> {
        let bound = self.bound.lock().unwrap().take();
        let reader = match bound {
            Some(reader) => reader,
            None => self.accessor.open(self.request.clone()).await?,
        };
        if reader.metadata().schema != self.schema {
            return Err("source schema changed; rebind the query".into());
        }
        Ok(reader)
    }
}
