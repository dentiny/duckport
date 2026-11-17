//! Authentication material passed as one owned security configuration.
use crate::{
    error::Result,
    utils::{self, optional_string},
};
use std::ffi::c_char;
#[repr(C)]
pub struct DuckportHttpHeader {
    pub name: *const c_char,
    pub value: *const c_char,
}
#[repr(C)]
#[derive(Default)]
pub struct DuckportSecurityOptions {
    pub access_key_id: *const c_char,
    pub secret_access_key: *const c_char,
    pub session_token: *const c_char,
    pub token: *const c_char,
    pub http_headers: *const DuckportHttpHeader,
    pub http_header_count: usize,
    pub allow_anonymous: i32,
}
// Deliberately no Debug implementation: authentication material must not be logged.
#[derive(Default)]
pub struct SecurityConfig {
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub token: Option<String>,
    pub http_headers: Vec<(String, String)>,
    pub allow_anonymous: bool,
}
impl DuckportSecurityOptions {
    /// # Safety
    /// All strings/arrays are live for this call. Returned settings own all material.
    pub(crate) unsafe fn read(&self) -> Result<SecurityConfig> {
        Ok(SecurityConfig {
            access_key_id: unsafe { optional_string(self.access_key_id) }?,
            secret_access_key: unsafe { optional_string(self.secret_access_key) }?,
            session_token: unsafe { optional_string(self.session_token) }?,
            token: unsafe { optional_string(self.token) }?,
            http_headers: unsafe {
                utils::ffi_slice(
                    self.http_headers,
                    self.http_header_count,
                    "null HTTP security headers",
                )
            }?
            .iter()
            .map(|h| unsafe { Ok((utils::string(h.name)?, utils::string(h.value)?)) })
            .collect::<Result<_>>()?,
            allow_anonymous: utils::boolean("allow_anonymous", self.allow_anonymous)?,
        })
    }
}
