use crate::{error::Result, validation};
use open_arrow_access::ReadOptions;
use std::ffi::{CStr, c_char};

#[repr(C)]
pub struct StorageOptions {
    pub endpoint: *const c_char,
    pub region: *const c_char,
    pub access_key_id: *const c_char,
    pub secret_access_key: *const c_char,
    pub session_token: *const c_char,
    pub token: *const c_char,
    pub revision: *const c_char,
    pub virtual_host_style: i32,
    pub allow_anonymous: i32,
}

#[repr(C)]
pub struct ScanOptions {
    pub batch_size: usize,
    /// -1 infers the CSV header; 0 disables it; 1 enables it.
    pub header: i32,
    pub record_path: *const c_char,
    pub format: *const c_char,
    pub path: *const c_char,
    pub config: *const c_char,
    pub split: *const c_char,
    pub allow_partial: i32,
}

pub struct StorageConfig {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub token: Option<String>,
    pub revision: Option<String>,
    pub virtual_host_style: bool,
    pub allow_anonymous: bool,
}

pub struct ScanConfig {
    pub read_options: ReadOptions,
    pub header: Option<bool>,
    pub record_path: Option<String>,
    pub format: Option<String>,
    pub path: Option<String>,
    pub config: Option<String>,
    pub split: Option<String>,
    pub allow_partial: bool,
}

impl StorageOptions {
    pub(crate) unsafe fn read(&self) -> Result<StorageConfig> {
        Ok(StorageConfig {
            endpoint: unsafe { optional_string(self.endpoint) }?,
            region: unsafe { optional_string(self.region) }?,
            access_key_id: unsafe { optional_string(self.access_key_id) }?,
            secret_access_key: unsafe { optional_string(self.secret_access_key) }?,
            session_token: unsafe { optional_string(self.session_token) }?,
            token: unsafe { optional_string(self.token) }?,
            revision: unsafe { optional_string(self.revision) }?,
            virtual_host_style: validation::boolean("virtual_host_style", self.virtual_host_style)?,
            allow_anonymous: validation::boolean("allow_anonymous", self.allow_anonymous)?,
        })
    }
}

impl ScanOptions {
    pub(crate) unsafe fn read(&self) -> Result<ScanConfig> {
        Ok(ScanConfig {
            read_options: ReadOptions {
                batch_size: validation::batch_size(self.batch_size)?,
                ..Default::default()
            },
            header: validation::optional_boolean("header", self.header)?,
            record_path: unsafe { optional_string(self.record_path) }?,
            format: unsafe { optional_string(self.format) }?,
            path: unsafe { optional_string(self.path) }?,
            config: unsafe { optional_string(self.config) }?,
            split: unsafe { optional_string(self.split) }?,
            allow_partial: validation::boolean("allow_partial", self.allow_partial)?,
        })
    }
}

unsafe fn optional_string(value: *const c_char) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(unsafe { CStr::from_ptr(value) }.to_str()?.to_owned()))
}
