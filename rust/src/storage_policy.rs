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
        }
    }
}

impl DuckportStoragePolicy {
    pub(crate) fn config(self) -> Result<StorageConfig> {
        crate::validation::validate_storage_policy(&self)?;
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
