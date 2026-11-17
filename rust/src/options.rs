use crate::storage_policy::DuckportStoragePolicy;
use crate::{error::Result, utils, utils::optional_string};
use open_arrow_access::ReadOptions;
use std::ffi::c_char;

#[repr(C)]
#[derive(Default)]
pub struct DuckportStorageOptions {
    pub endpoint: *const c_char,
    pub region: *const c_char,
    pub virtual_host_style: i32,
    pub policy: DuckportStoragePolicy,
    pub security: crate::security::DuckportSecurityOptions,
}

#[repr(C)]
pub struct DuckportScanOptions {
    /// Target rows per Arrow record batch; SQL default: 8192.
    pub batch_size: usize,
}

#[derive(Default)]
pub struct StorageConfig {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub virtual_host_style: bool,
    pub policy: DuckportStoragePolicy,
    pub security: crate::security::SecurityConfig,
}

impl DuckportStorageOptions {
    pub(crate) unsafe fn read(&self) -> Result<StorageConfig> {
        self.policy.config()?;
        Ok(StorageConfig {
            endpoint: unsafe { optional_string(self.endpoint) }?,
            region: unsafe { optional_string(self.region) }?,
            virtual_host_style: utils::boolean("virtual_host_style", self.virtual_host_style)?,
            policy: self.policy,
            security: unsafe { self.security.read() }?,
        })
    }
}

impl DuckportScanOptions {
    pub(crate) fn read(&self) -> Result<ReadOptions> {
        Ok(ReadOptions {
            batch_size: utils::batch_size(self.batch_size)?,
            ..Default::default()
        })
    }
}
