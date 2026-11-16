//! Shared single-page OAI-PMH adapter for the upstream provider trait.
use crate::{
    error::Result, options::StorageConfig, source::DuckportSource as AsyncSource,
    validation::optional_string,
};
use open_arrow_access::{
    ReadOptions, ReadRequest,
    file_format::xml::{NamespaceBinding, XmlFormat},
    service::{
        common::Connection,
        oai::{OaiProvider, OaiProviderService},
    },
};
use std::ffi::c_char;

#[repr(C)]
pub struct OaiOptions {
    pub endpoint: *const c_char,
    pub from_date: *const c_char,
    pub until_date: *const c_char,
    pub set_spec: *const c_char,
    pub record_path: *const c_char,
}

pub struct OaiSelection {
    endpoint: Option<String>,
    from_date: Option<String>,
    until_date: Option<String>,
    set_spec: Option<String>,
    record_path: Option<String>,
}

impl OaiOptions {
    /// # Safety
    /// Optional strings are valid NUL-terminated UTF-8 for this call.
    pub unsafe fn read(&self) -> Result<OaiSelection> {
        Ok(OaiSelection {
            endpoint: unsafe { optional_string(self.endpoint) }?,
            from_date: unsafe { optional_string(self.from_date) }?,
            until_date: unsafe { optional_string(self.until_date) }?,
            set_spec: unsafe { optional_string(self.set_spec) }?,
            record_path: unsafe { optional_string(self.record_path) }?,
        })
    }
}

pub async fn open<P: OaiProvider>(
    locator: String,
    storage_options: StorageConfig,
    scan_options: ReadOptions,
    selection: OaiSelection,
) -> Result<AsyncSource> {
    let OaiSelection {
        endpoint,
        from_date,
        until_date,
        set_spec,
        record_path,
    } = selection;
    let endpoint = endpoint.as_deref().unwrap_or(P::ORIGIN);
    crate::validation::validate_oai_selectors(
        &locator,
        from_date.as_deref(),
        until_date.as_deref(),
        set_spec.as_deref(),
    )?;
    let connection = Connection::new(endpoint)?;
    let connection = if let Some(token) = storage_options.token {
        connection.with_header("authorization", &format!("Bearer {token}"))?
    } else {
        connection
    };
    let service = OaiProviderService::<P>::with_connection(connection)?;
    let page = if locator == "records" {
        service
            .records(
                from_date.as_deref(),
                until_date.as_deref(),
                set_spec.as_deref(),
            )
            .await?
    } else {
        service.resume(&locator).await?
    };
    let format = match record_path {
        Some(record_path) => XmlFormat::new(record_path).with_namespaces(&[NamespaceBinding {
            prefix: "oai",
            uri: "http://www.openarchives.org/OAI/2.0/",
        }]),
        None => service.inferred_record_format(),
    };
    let records = page.records(format);
    AsyncSource::new(
        records,
        ReadRequest::new(locator).with_options(scan_options),
    )
    .await
}
