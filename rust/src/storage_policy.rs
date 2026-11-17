//! Per-query policy shared by OpenDAL operators and provider HTTP transports.
use crate::error::Result;
use open_arrow_access::storage::{RetryConfig, StorageConfig, TimeoutConfig};
use std::time::{Duration, Instant};

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
            timeout_ms: 30_000,
            io_timeout_ms: 30_000,
            retry_max_times: 5,
            retry_min_delay_ms: 500,
            retry_max_delay_ms: 10_000,
            retry_factor: 2.0,
        }
    }
}

impl DuckportStoragePolicy {
    pub(crate) fn config(self) -> Result<StorageConfig> {
        for (name, value) in [
            ("timeout_ms", self.timeout_ms),
            ("io_timeout_ms", self.io_timeout_ms),
            ("retry_min_delay_ms", self.retry_min_delay_ms),
            ("retry_max_delay_ms", self.retry_max_delay_ms),
        ] {
            if value == 0 {
                return Err(format!("conversion_storage_{name} must be positive").into());
            }
            if Instant::now()
                .checked_add(Duration::from_millis(value))
                .is_none()
            {
                return Err(format!("conversion_storage_{name} is too large").into());
            }
        }
        if self.retry_max_delay_ms < self.retry_min_delay_ms {
            return Err(
                "conversion_storage_retry_max_delay_ms must be at least retry_min_delay_ms".into(),
            );
        }
        if !self.retry_factor.is_finite()
            || self.retry_factor < 1.0
            || self.retry_factor > f32::MAX as f64
        {
            return Err(
                "conversion_storage_retry_factor must be finite and between 1 and f32::MAX".into(),
            );
        }
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
