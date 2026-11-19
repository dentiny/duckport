//! Per-query policy shared by OpenDAL operators and provider HTTP transports.
use crate::error::Result;
use open_arrow_access::storage::{RetryConfig, StorageConfig, TimeoutConfig};
use std::time::Duration;

const DEFAULT_STORAGE_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_STORAGE_IO_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_STORAGE_RETRY_MAX_TIMES: u64 = 5;
const DEFAULT_STORAGE_RETRY_MIN_DELAY_MS: u64 = 500;
const DEFAULT_STORAGE_RETRY_MAX_DELAY_MS: u64 = 10_000;
const DEFAULT_STORAGE_RETRY_FACTOR: f64 = 2.0;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DuckportStoragePolicy {
    pub timeout_ms: u64,
    pub io_timeout_ms: u64,
    pub retry_max_times: u64,
    pub retry_min_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub retry_factor: f64,
    pub io_request_size: u64,
}

impl Default for DuckportStoragePolicy {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_STORAGE_TIMEOUT_MS,
            io_timeout_ms: DEFAULT_STORAGE_IO_TIMEOUT_MS,
            retry_max_times: DEFAULT_STORAGE_RETRY_MAX_TIMES,
            retry_min_delay_ms: DEFAULT_STORAGE_RETRY_MIN_DELAY_MS,
            retry_max_delay_ms: DEFAULT_STORAGE_RETRY_MAX_DELAY_MS,
            retry_factor: DEFAULT_STORAGE_RETRY_FACTOR,
            io_request_size: 2 * 1024 * 1024,
        }
    }
}

impl DuckportStoragePolicy {
    pub(crate) fn io_request_size(self) -> Result<std::num::NonZeroUsize> {
        crate::validation::required(
            std::num::NonZeroUsize::new(usize::try_from(self.io_request_size)?),
            "conversion_storage_io_request_size must be positive",
        )
    }

    pub(crate) fn byte_options(self) -> Result<open_arrow_access::ByteReadOptions> {
        Ok(open_arrow_access::ByteReadOptions {
            io_chunk_size: Some(self.io_request_size()?),
            ..Default::default()
        })
    }

    pub(crate) fn config(self) -> Result<StorageConfig> {
        crate::validation::validate_storage_policy(&self)?;
        self.io_request_size()?;
        Ok(StorageConfig {
            timeout: Some(TimeoutConfig {
                operation_timeout: Duration::from_millis(self.timeout_ms),
                io_timeout: Duration::from_millis(self.io_timeout_ms),
            }),
            retry: Some(RetryConfig {
                max_retries: usize::try_from(self.retry_max_times)?,
                min_delay: Duration::from_millis(self.retry_min_delay_ms),
                max_delay: Duration::from_millis(self.retry_max_delay_ms),
                factor: self.retry_factor as f32,
                jitter: true,
            }),
        })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn duckport_default_storage_policy() -> DuckportStoragePolicy {
    DuckportStoragePolicy::default()
}
