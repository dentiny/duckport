//! Per-source observations shared with OpenDAL and the Arrow stream.
use arrow_array::{RecordBatch, RecordBatchReader};
use arrow_schema::{ArrowError, SchemaRef};
use open_arrow_access::metrics::{MetricEvent, MetricValue, MetricsExporter};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering::Relaxed},
};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct DuckportReadMetrics {
    pub bytes_read: u64,
    pub rows_read: u64,
    pub total_bytes: u64,
    pub progress_bytes: u64,
    pub total_bytes_known: bool,
    pub finished: bool,
}

#[derive(Default)]
struct State {
    bytes: AtomicU64,
    delivered: AtomicU64,
    storage_observed: AtomicBool,
    rows: AtomicU64,
    total: AtomicU64,
    known: AtomicBool,
    finished: AtomicBool,
    started: AtomicBool,
    baseline: AtomicU64,
}

/// Constant-space counters; snapshots never perform I/O or retain source URLs.
#[derive(Clone, Default)]
pub(crate) struct ReadMetrics(Arc<State>);

impl ReadMetrics {
    pub(crate) fn observe_storage(&self) {
        self.0.storage_observed.store(true, Relaxed);
    }

    pub(crate) fn set_total_bytes(&self, bytes: u64) {
        self.0.total.store(bytes, Relaxed);
        self.0.known.store(true, Relaxed);
    }

    pub(crate) fn begin_scan(&self) {
        if self.0.started.swap(true, Relaxed) {
            self.0
                .baseline
                .store(self.0.delivered.load(Relaxed), Relaxed);
        }
        self.0.finished.store(false, Relaxed);
    }

    pub(crate) fn snapshot(&self) -> DuckportReadMetrics {
        let bytes_read = if self.0.storage_observed.load(Relaxed) {
            self.0.bytes.load(Relaxed)
        } else {
            self.0.delivered.load(Relaxed)
        };
        DuckportReadMetrics {
            bytes_read,
            rows_read: self.0.rows.load(Relaxed),
            total_bytes: self.0.total.load(Relaxed),
            progress_bytes: self
                .0
                .delivered
                .load(Relaxed)
                .saturating_sub(self.0.baseline.load(Relaxed)),
            total_bytes_known: self.0.known.load(Relaxed),
            finished: self.0.finished.load(Relaxed),
        }
    }
}

impl MetricsExporter for ReadMetrics {
    fn record(&self, event: MetricEvent) {
        if let ("oaa_bytes_delivered_total", MetricValue::Counter(bytes)) =
            (event.name, event.value)
        {
            self.0.delivered.fetch_add(bytes, Relaxed);
        }
        if let ("opendal_operation_bytes", "read", MetricValue::Counter(bytes)) =
            (event.name, event.operation, event.value)
        {
            self.0.bytes.fetch_add(bytes, Relaxed);
        }
    }
}

/// Observe the final Arrow boundary, including zero-column scans and EOF.
/// Errors and early drops never mark the scan complete.
pub(crate) struct ObservedReader<R> {
    inner: R,
    metrics: ReadMetrics,
    terminal: bool,
}
impl<R> ObservedReader<R> {
    pub(crate) fn new(inner: R, metrics: ReadMetrics) -> Self {
        Self {
            inner,
            metrics,
            terminal: false,
        }
    }
}
impl<R: RecordBatchReader> Iterator for ObservedReader<R> {
    type Item = Result<RecordBatch, ArrowError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.terminal {
            return None;
        }
        let result = self.inner.next();
        match &result {
            Some(Ok(batch)) => {
                self.metrics
                    .0
                    .rows
                    .fetch_add(batch.num_rows() as u64, Relaxed);
            }
            None => {
                self.terminal = true;
                self.metrics.0.finished.store(true, Relaxed);
            }
            Some(Err(_)) => {
                self.terminal = true;
            }
        }
        result
    }
}
impl<R: RecordBatchReader> RecordBatchReader for ObservedReader<R> {
    fn schema(&self) -> SchemaRef {
        self.inner.schema()
    }
}

/// Observe a single encoded object before decompression. Its original metadata
/// supplies the progress denominator without an extra stat request.
pub(crate) struct ObservedBytes<A> {
    inner: A,
    metrics: ReadMetrics,
}
impl<A> ObservedBytes<A> {
    pub(crate) fn new(inner: A, metrics: ReadMetrics) -> Self {
        Self { inner, metrics }
    }
}
#[open_arrow_access::async_trait]
impl<A: open_arrow_access::ByteAccessor> open_arrow_access::Accessor for ObservedBytes<A> {
    type Request = open_arrow_access::ByteReadRequest;
    type Reader = dyn open_arrow_access::ByteReader;
    fn capability(&self) -> open_arrow_access::Capability {
        self.inner.capability()
    }
    async fn metadata(
        &self,
        request: Self::Request,
    ) -> open_arrow_access::Result<open_arrow_access::ByteMetadata> {
        self.inner.metadata(request).await
    }
    async fn inspect_metadata(
        &self,
        request: Self::Request,
    ) -> open_arrow_access::Result<Option<open_arrow_access::ByteMetadata>> {
        self.inner.inspect_metadata(request).await
    }
    async fn open(&self, request: Self::Request) -> open_arrow_access::Result<Box<Self::Reader>> {
        use open_arrow_access::metrics::InstrumentReader;
        let reader = self.inner.open(request).await?;
        if let Some(total) = reader.metadata().content_length {
            self.metrics.set_total_bytes(total);
        }
        Ok(reader.instrument(self.metrics.clone()))
    }
}
#[open_arrow_access::async_trait]
impl<A: open_arrow_access::ByteAccessor> open_arrow_access::ByteAccessor for ObservedBytes<A> {
    async fn list(
        &self,
        request: open_arrow_access::ListRequest,
    ) -> open_arrow_access::Result<open_arrow_access::ObjectStream> {
        self.inner.list(request).await
    }
}
