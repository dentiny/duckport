use crate::{
    ffi::{DuckportSource, open_reader},
    options::{DuckportScanOptions, DuckportStorageOptions, StorageConfig, optional_string},
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::{
    ReadRequest,
    file_format::xml::{NamespaceBinding, XmlFormat},
    service::{common::Connection, doab::DoabService},
};
use std::ffi::c_char;

#[repr(C)]
pub struct DoabOptions {
    pub endpoint: *const c_char,
    pub from_date: *const c_char,
    pub until_date: *const c_char,
    pub set_spec: *const c_char,
    pub record_path: *const c_char,
}

/// # Safety
/// Inputs are borrowed for this call; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_doab_open(
    locator: *const c_char,
    storage_options: &DuckportStorageOptions,
    scan_options: &DuckportScanOptions,
    reader_options: &DoabOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    let open = |locator: String, storage_options: StorageConfig, scan_options| async move {
        let endpoint = unsafe {
            optional_string(reader_options.endpoint)?
                .unwrap_or_else(|| "https://directory.doabooks.org".into())
        };
        let from_date = unsafe { optional_string(reader_options.from_date)? };
        let until_date = unsafe { optional_string(reader_options.until_date)? };
        let set_spec = unsafe { optional_string(reader_options.set_spec)? };
        let record_path = unsafe {
            optional_string(reader_options.record_path)?
                .unwrap_or_else(|| "/oai:OAI-PMH/oai:ListRecords/oai:record".into())
        };
        if locator != "records"
            && (from_date.is_some() || until_date.is_some() || set_spec.is_some())
        {
            return Err("resumption tokens cannot be combined with date/set selectors".into());
        }
        let connection = Connection::new(&endpoint)?;
        let connection = if let Some(token) = storage_options.token {
            connection.with_header("authorization", &format!("Bearer {token}"))?
        } else {
            connection
        };
        let service = DoabService::with_connection(connection)?;
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
        let records =
            page.records(
                XmlFormat::new(record_path).with_namespaces(&[NamespaceBinding {
                    prefix: "oai",
                    uri: "http://www.openarchives.org/OAI/2.0/",
                }]),
            );
        AsyncSource::new(
            records,
            ReadRequest::new(locator).with_options(scan_options),
        )
        .await
    };
    unsafe { open_reader(locator, storage_options, scan_options, output, open) }
}
