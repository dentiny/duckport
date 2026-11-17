use crate::error::Result;
use arrow_schema::SchemaRef;
use open_arrow_access::{
    Accessor, OuterCompression, PreparedSource, ReadOptions, ReadOptionsProvider, ReadRequest,
    ReaderFactory, RecordReader, file_accessor, storage::opendal,
};

pub struct DuckportSource {
    source: PreparedSource,
    options: ReadOptions,
}

impl DuckportSource {
    pub async fn new<A>(accessor: A, request: A::Request) -> Result<Self>
    where
        A: Accessor<Reader = dyn RecordReader> + 'static,
        A::Request: Clone + Sync + ReadOptionsProvider + 'static,
    {
        let options = request.read_options().clone();
        Ok(Self {
            source: PreparedSource::prepare(accessor, request).await?,
            options,
        })
    }

    pub fn schema(&self) -> SchemaRef {
        self.source.schema()
    }

    pub async fn reader(&self, projection: Vec<String>) -> Result<Box<dyn RecordReader>> {
        let mut options = self.options.clone();
        // Reuse the reader retained at bind for a complete, schema-order scan.
        // Empty means zero columns (COUNT(*)), not SELECT *.
        options.projection = if projection.iter().map(String::as_str).eq(self
            .source
            .schema()
            .fields()
            .iter()
            .map(|field| field.name().as_str()))
        {
            None
        } else {
            Some(projection)
        };
        Ok(self.source.scan(options).await?)
    }
}

pub(crate) async fn open_file_source<F>(
    operator: opendal::Operator,
    request: ReadRequest,
    format: F,
    compression: OuterCompression,
) -> Result<DuckportSource>
where
    F: ReaderFactory<Request = ReadRequest, Reader = dyn RecordReader> + 'static,
{
    DuckportSource::new(file_accessor(operator, format, compression), request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray};
    use open_arrow_access::{
        OrderRequirement, RecordCapability, RecordMetadata, StreamReader, async_trait,
    };
    use std::sync::{Arc, Mutex};

    struct RecordingAccessor {
        batch: RecordBatch,
        projections: Arc<Mutex<Vec<Option<Vec<String>>>>>,
    }

    #[async_trait]
    impl Accessor for RecordingAccessor {
        type Request = ReadRequest;
        type Reader = dyn RecordReader;

        fn capability(&self) -> RecordCapability {
            RecordCapability::sequential(OrderRequirement::PreserveGlobal)
        }

        async fn open(
            &self,
            request: ReadRequest,
        ) -> open_arrow_access::Result<Box<dyn RecordReader>> {
            assert_eq!(request.options.batch_size.get(), 2);
            self.projections
                .lock()
                .unwrap()
                .push(request.options.projection.clone());
            let (_, indices) = request.options.resolve_projection(&self.batch.schema())?;
            let batch = match indices {
                Some(indices) => self.batch.project(&indices).unwrap(),
                None => self.batch.clone(),
            };
            let metadata = RecordMetadata {
                schema: batch.schema(),
                source: None,
            };
            Ok(Box::new(StreamReader::records(
                futures::stream::iter([Ok(batch)]),
                metadata,
                OrderRequirement::PreserveGlobal,
            )))
        }
    }

    #[tokio::test]
    async fn forwards_each_projection_and_preserves_zero_column_row_counts() {
        let batch = RecordBatch::try_from_iter([
            ("id", Arc::new(Int64Array::from(vec![1, 2])) as ArrayRef),
            (
                "body",
                Arc::new(StringArray::from(vec!["one", "two"])) as ArrayRef,
            ),
        ])
        .unwrap();
        let projections = Arc::new(Mutex::new(Vec::new()));
        let source = DuckportSource::new(
            RecordingAccessor {
                batch: batch.clone(),
                projections: projections.clone(),
            },
            ReadRequest::new("fixture").with_options(ReadOptions {
                batch_size: 2.try_into().unwrap(),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
        assert_eq!(source.schema(), batch.schema());
        let mut body = source.reader(vec!["body".into()]).await.unwrap();
        let mut reordered = source
            .reader(vec!["body".into(), "id".into()])
            .await
            .unwrap();
        let mut count = source.reader(vec![]).await.unwrap();
        let mut ids = source.reader(vec!["id".into()]).await.unwrap();
        assert_eq!(
            body.next().await.unwrap().unwrap(),
            batch.project(&[1]).unwrap()
        );
        assert_eq!(
            reordered.next().await.unwrap().unwrap(),
            batch.project(&[1, 0]).unwrap()
        );
        let counted = count.next().await.unwrap().unwrap();
        assert_eq!((counted.num_rows(), counted.num_columns()), (2, 0));
        assert_eq!(
            ids.next().await.unwrap().unwrap(),
            batch.project(&[0]).unwrap()
        );
        assert_eq!(
            *projections.lock().unwrap(),
            vec![
                None,
                Some(vec!["body".into()]),
                Some(vec!["body".into(), "id".into()]),
                Some(vec![]),
                Some(vec!["id".into()]),
            ]
        );
        assert!(source.reader(vec!["missing".into()]).await.is_err());
        assert!(source.reader(vec!["id".into(), "id".into()]).await.is_err());
        assert_eq!(projections.lock().unwrap().len(), 5);
        // SELECT * consumes the unpolled bind reader without opening again.
        let mut all = source
            .reader(vec!["id".into(), "body".into()])
            .await
            .unwrap();
        assert_eq!(all.next().await.unwrap().unwrap(), batch);
        assert_eq!(projections.lock().unwrap().len(), 5);
    }
}
