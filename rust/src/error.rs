use arrow_array::RecordBatch;
use arrow_schema::ArrowError;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) fn read_batch(
    operation: impl FnOnce() -> open_arrow_access::Result<Option<RecordBatch>>,
) -> std::result::Result<Option<RecordBatch>, ArrowError> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(batch)) => Ok(batch),
        Ok(Err(error)) => Err(ArrowError::ExternalError(Box::new(error))),
        Err(_) => Err(ArrowError::ComputeError("source reader panicked".into())),
    }
}
