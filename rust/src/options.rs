use crate::{error::Result, validation};
use open_arrow_access::ReadOptions;
use std::ffi::{CStr, c_char};

#[repr(C)]
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

pub(crate) unsafe fn optional_string(value: *const c_char) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(unsafe { CStr::from_ptr(value) }.to_str()?.to_owned()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttachmentFormat {
    Binary,
    Csv,
    Tsv,
    Json,
    Jsonl,
    Parquet,
    Xml,
}

impl std::str::FromStr for AttachmentFormat {
    type Err = Box<dyn std::error::Error + Send + Sync>;

    fn from_str(value: &str) -> Result<Self> {
        validation::parse_attachment_format(value)
    }
}
