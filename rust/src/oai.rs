//! Shared OAI-PMH adapter with demand-driven continuation scans.
use crate::{
    error::Result, options::StorageConfig, source::DuckportSource as AsyncSource,
    utils::optional_string,
};
use open_arrow_access::{
    Accessor, AccessorExt, ReadRequest, RecordCapability, RecordReader, async_trait,
    file_format::xml::{NamespaceBinding, XmlFormat},
    layers::LimitLayer,
    service::oai::{OaiHarvestRequest, OaiHarvester, OaiProvider, OaiProviderService},
};
use std::{ffi::c_char, num::NonZeroUsize};

#[repr(C)]
pub struct OaiOptions {
    pub endpoint: *const c_char,
    pub from_date: *const c_char,
    pub until_date: *const c_char,
    pub set_spec: *const c_char,
    pub record_path: *const c_char,
    pub max_pages: i64,
    pub max_records: i64,
}

pub struct OaiSelection {
    endpoint: Option<String>,
    from_date: Option<String>,
    until_date: Option<String>,
    set_spec: Option<String>,
    record_path: Option<String>,
    max_pages: Option<NonZeroUsize>,
    max_records: Option<usize>,
}

impl OaiOptions {
    /// # Safety
    /// Optional strings are valid NUL-terminated UTF-8 for this call.
    pub unsafe fn read(&self) -> Result<OaiSelection> {
        let limit = |name, value: i64| -> Result<Option<usize>> {
            if value < 0 {
                return Err(format!("{name} must be positive").into());
            }
            if value == 0 {
                return Ok(None);
            }
            Ok(Some(usize::try_from(value)?))
        };
        let max_pages = limit("max_pages", self.max_pages)?.and_then(NonZeroUsize::new);
        let max_records = limit("max_records", self.max_records)?;
        Ok(OaiSelection {
            endpoint: unsafe { optional_string(self.endpoint) }?,
            from_date: unsafe { optional_string(self.from_date) }?,
            until_date: unsafe { optional_string(self.until_date) }?,
            set_spec: unsafe { optional_string(self.set_spec) }?,
            record_path: unsafe { optional_string(self.record_path) }?,
            max_pages,
            max_records,
        })
    }
}

pub(crate) async fn open<P: OaiProvider>(
    locator: String,
    storage_options: StorageConfig,
    scan_options: crate::options::ScanOptions,
    selection: OaiSelection,
) -> Result<AsyncSource> {
    let OaiSelection {
        endpoint,
        from_date,
        until_date,
        set_spec,
        record_path,
        max_pages,
        max_records,
    } = selection;
    let endpoint = endpoint.as_deref().unwrap_or(P::ORIGIN);
    crate::validation::validate_oai_selectors(
        &locator,
        from_date.as_deref(),
        until_date.as_deref(),
        set_spec.as_deref(),
    )?;
    let connection = crate::provider_auth::connection(
        endpoint,
        &storage_options,
        crate::provider_auth::HttpAuth::Bearer,
    )?;
    let service = OaiProviderService::<P>::with_connection(connection)?;
    let format = match record_path {
        Some(path) => XmlFormat::new(path).with_namespaces(&[NamespaceBinding {
            prefix: "oai",
            uri: "http://www.openarchives.org/OAI/2.0/",
        }]),
        None => service.record_format(),
    };
    let mut selection = OaiHarvestRequest::new(
        ReadRequest::new(locator.clone()).with_byte_options(scan_options.bytes),
    );
    selection.from = from_date;
    selection.until = until_date;
    selection.set = set_spec;
    selection.resumption_token = (locator != "records").then(|| locator.clone());
    selection.max_pages = max_pages;
    let accessor = SelectedHarvest {
        accessor: service.harvest().with_format(format),
        selection,
    };
    let request = scan_options.request(ReadRequest::new(locator));
    match max_records {
        Some(rows) => AsyncSource::new(accessor.layer(LimitLayer::new(rows as u64)), request).await,
        None => AsyncSource::new(accessor, request).await,
    }
}

// OaiHarvestRequest in published access 0.12.1 does not implement
// ReadOptionsProvider, so it cannot use upstream SelectionAccessor yet.
// Keep continuation handling in OaiHarvester and bind only its SQL selectors.
struct SelectedHarvest {
    accessor: OaiHarvester,
    selection: OaiHarvestRequest,
}

#[async_trait]
impl Accessor for SelectedHarvest {
    type Request = ReadRequest;
    type Reader = dyn RecordReader;

    fn capability(&self) -> RecordCapability {
        self.accessor.capability()
    }

    async fn open(&self, request: ReadRequest) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        let mut selection = self.selection.clone();
        selection.read.options = request.options;
        selection.read.byte_options = request.byte_options;
        self.accessor.open(selection).await
    }
}
