use crate::{error::Result, ffi::common::optional_string, validation};
use open_arrow_access::ReadOptions;
use std::ffi::c_char;

#[repr(C)]
#[derive(Default)]
pub struct DuckportStorageOptions {
    pub endpoint: *const c_char,
    pub region: *const c_char,
    pub access_key_id: *const c_char,
    pub secret_access_key: *const c_char,
    pub session_token: *const c_char,
    pub token: *const c_char,
    pub virtual_host_style: i32,
    pub allow_anonymous: i32,
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
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub token: Option<String>,
    pub virtual_host_style: bool,
    pub allow_anonymous: bool,
}

impl DuckportStorageOptions {
    pub(crate) unsafe fn read(&self) -> Result<StorageConfig> {
        Ok(StorageConfig {
            endpoint: unsafe { optional_string(self.endpoint) }?,
            region: unsafe { optional_string(self.region) }?,
            access_key_id: unsafe { optional_string(self.access_key_id) }?,
            secret_access_key: unsafe { optional_string(self.secret_access_key) }?,
            session_token: unsafe { optional_string(self.session_token) }?,
            token: unsafe { optional_string(self.token) }?,
            virtual_host_style: validation::boolean("virtual_host_style", self.virtual_host_style)?,
            allow_anonymous: validation::boolean("allow_anonymous", self.allow_anonymous)?,
        })
    }
}

impl DuckportScanOptions {
    pub(crate) fn read(&self) -> Result<ReadOptions> {
        Ok(ReadOptions {
            batch_size: validation::batch_size(self.batch_size)?,
            ..Default::default()
        })
    }
}
