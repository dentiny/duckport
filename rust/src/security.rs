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
// Deliberately no Debug implementations: credentials must not be logged.
#[derive(Default)]
pub struct HttpCredentials {
    token: Option<String>,
    headers: http::HeaderMap,
}
impl HttpCredentials {
    pub(crate) fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
    pub(crate) fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }
    pub(crate) fn download_headers(&self) -> Result<http::HeaderMap> {
        let mut headers = self.headers.clone();
        if let Some(token) = &self.token {
            if headers.contains_key(http::header::AUTHORIZATION) {
                return Err("choose BEARER_TOKEN or an Authorization header".into());
            }
            let mut value = http::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| "invalid HTTP bearer token")?;
            value.set_sensitive(true);
            headers.insert(http::header::AUTHORIZATION, value);
        }
        Ok(headers)
    }
}
pub enum S3Credentials {
    Anonymous,
    Static {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
}
#[derive(Default)]
pub enum SecurityConfig {
    #[default]
    None,
    Http(HttpCredentials),
    S3(S3Credentials),
}
impl SecurityConfig {
    pub(crate) fn http(&self) -> Result<Option<&HttpCredentials>> {
        match self {
            Self::None => Ok(None),
            Self::Http(credentials) => Ok(Some(credentials)),
            Self::S3(_) => Err("S3 credentials cannot authenticate an HTTP provider".into()),
        }
    }
    pub(crate) fn s3(&self) -> Result<Option<&S3Credentials>> {
        match self {
            Self::None => Ok(None),
            Self::S3(credentials) => Ok(Some(credentials)),
            Self::Http(_) => Err("HTTP credentials cannot authenticate S3 storage".into()),
        }
    }
    fn into_http(self) -> Result<HttpCredentials> {
        match self {
            Self::None => Ok(HttpCredentials::default()),
            Self::Http(credentials) => Ok(credentials),
            Self::S3(_) => Err("HTTP source secrets cannot contain S3 credentials".into()),
        }
    }
}
#[repr(C)]
pub struct DuckportHttpSecret {
    pub scope: *const c_char,
    pub security: DuckportSecurityOptions,
}

// A scoped HTTP secret cannot carry AWS keys or storage policy. Headers are
// parsed at the boundary; token interpretation belongs to the consuming adapter.
pub(crate) struct ScopedHttpCredentials {
    pub scope: String,
    pub credentials: HttpCredentials,
}

pub(crate) unsafe fn read_http_secret(
    scope: *const c_char,
    raw: &DuckportSecurityOptions,
) -> Result<ScopedHttpCredentials> {
    Ok(ScopedHttpCredentials {
        scope: unsafe { utils::string(scope) }?,
        credentials: unsafe { raw.read() }?.into_http()?,
    })
}
pub(crate) unsafe fn read_http_secrets(
    raw: *const DuckportHttpSecret,
    count: usize,
) -> Result<Vec<ScopedHttpCredentials>> {
    unsafe { utils::ffi_slice(raw, count, "null HTTP source secrets") }?
        .iter()
        .map(|secret| unsafe { read_http_secret(secret.scope, &secret.security) })
        .collect()
}

impl DuckportSecurityOptions {
    /// # Safety
    /// All strings/arrays are live for this call. Returned credentials own all material.
    pub(crate) unsafe fn read(&self) -> Result<SecurityConfig> {
        let access_key_id = unsafe { optional_string(self.access_key_id) }?;
        let secret_access_key = unsafe { optional_string(self.secret_access_key) }?;
        let session_token = unsafe { optional_string(self.session_token) }?;
        let token = unsafe { optional_string(self.token) }?;
        let anonymous = utils::boolean("allow_anonymous", self.allow_anonymous)?;
        let has_s3 = access_key_id.is_some()
            || secret_access_key.is_some()
            || session_token.is_some()
            || anonymous;
        let has_http = token.is_some() || self.http_header_count != 0;
        if has_s3 && has_http {
            return Err("HTTP and S3 credentials must not be combined".into());
        }
        if has_s3 {
            return match (access_key_id, secret_access_key, session_token, anonymous) {
                (None, None, None, true) => Ok(SecurityConfig::S3(S3Credentials::Anonymous)),
                (Some(key), Some(secret), session_token, false)
                    if !key.is_empty() && !secret.is_empty() =>
                {
                    Ok(SecurityConfig::S3(S3Credentials::Static {
                        access_key_id: key,
                        secret_access_key: secret,
                        session_token,
                    }))
                }
                (_, _, _, true) => Err("anonymous S3 access cannot include credentials".into()),
                _ => Err("S3 requires both a nonempty access key and a secret key".into()),
            };
        }
        let mut headers = http::HeaderMap::new();
        for header in unsafe {
            utils::ffi_slice(
                self.http_headers,
                self.http_header_count,
                "null HTTP security headers",
            )
        }? {
            let name = unsafe { utils::string(header.name) }?;
            let name = http::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| "invalid HTTP credential header name")?;
            if matches!(
                name.as_str(),
                "host" | "content-length" | "range" | "connection" | "transfer-encoding"
            ) {
                return Err("HTTP credential headers must not override transport headers".into());
            }
            if headers.contains_key(&name) {
                return Err("duplicate HTTP credential header".into());
            }
            let value = unsafe { utils::string(header.value) }?;
            let mut value = http::HeaderValue::from_str(&value)
                .map_err(|_| "invalid HTTP credential header value")?;
            value.set_sensitive(true);
            headers.insert(name, value);
        }
        if has_http {
            Ok(SecurityConfig::Http(HttpCredentials { token, headers }))
        } else {
            Ok(SecurityConfig::None)
        }
    }
}
