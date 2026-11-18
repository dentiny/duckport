//! Shared OAI-PMH adapter with opt-in, demand-driven continuation scans.
use crate::{
    error::Result, options::StorageConfig, source::DuckportSource as AsyncSource,
    utils::optional_string,
};
use futures::stream;
use open_arrow_access::{
    Accessor, ReadOptions, ReadRequest, RecordCapability, RecordReader, StreamReader, async_trait,
    file_format::xml::{NamespaceBinding, XmlFormat},
    service::{
        common::Connection,
        oai::{OaiHarvestRequest, OaiHarvester, OaiProvider, OaiProviderService},
    },
};
use std::{ffi::c_char, num::NonZeroUsize};

#[repr(C)]
pub struct OaiOptions {
    pub endpoint: *const c_char,
    pub from_date: *const c_char,
    pub until_date: *const c_char,
    pub set_spec: *const c_char,
    pub record_path: *const c_char,
    pub auto_paginate: i32,
    pub max_pages: i64,
    pub max_records: i64,
}

pub struct OaiSelection {
    endpoint: Option<String>,
    from_date: Option<String>,
    until_date: Option<String>,
    set_spec: Option<String>,
    record_path: Option<String>,
    auto_paginate: bool,
    max_pages: Option<NonZeroUsize>,
    max_records: Option<usize>,
}

impl OaiOptions {
    /// # Safety
    /// Optional strings are valid NUL-terminated UTF-8 for this call.
    pub unsafe fn read(&self) -> Result<OaiSelection> {
        crate::validation::validate_boolean("auto_paginate", self.auto_paginate)?;
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
        if self.auto_paginate == 0 && (max_pages.is_some() || max_records.is_some()) {
            return Err("max_pages and max_records require auto_paginate := true".into());
        }
        Ok(OaiSelection {
            endpoint: unsafe { optional_string(self.endpoint) }?,
            from_date: unsafe { optional_string(self.from_date) }?,
            until_date: unsafe { optional_string(self.until_date) }?,
            set_spec: unsafe { optional_string(self.set_spec) }?,
            record_path: unsafe { optional_string(self.record_path) }?,
            auto_paginate: self.auto_paginate != 0,
            max_pages,
            max_records,
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
        auto_paginate,
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
    let connection =
        Connection::new(endpoint)?.with_http_config(storage_options.policy.config()?)?;
    let connection = crate::provider_auth::apply(
        connection,
        storage_options.security.http()?,
        crate::provider_auth::HttpAuth::Bearer,
    )?;
    let service = OaiProviderService::<P>::with_connection(connection)?;
    if auto_paginate {
        let format = match record_path {
            Some(path) => XmlFormat::new(path).with_namespaces(&[NamespaceBinding {
                prefix: "oai",
                uri: "http://www.openarchives.org/OAI/2.0/",
            }]),
            None => service.record_format(),
        };
        let mut selection = OaiHarvestRequest::new(ReadRequest::new(locator.clone()));
        selection.from = from_date;
        selection.until = until_date;
        selection.set = set_spec;
        selection.resumption_token = (locator != "records").then(|| locator.clone());
        selection.max_pages = max_pages;
        let accessor = HarvestAccessor {
            harvester: service.harvest().with_format(format),
            selection,
            max_records,
        };
        return AsyncSource::new(
            accessor,
            ReadRequest::new(locator).with_options(scan_options),
        )
        .await;
    }
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
        None if page.record_count == 0 => service.record_format(),
        None => service.inferred_record_format(),
    };
    let records = page.records(format);
    AsyncSource::new(
        records,
        ReadRequest::new(locator).with_options(scan_options),
    )
    .await
}

// Keep provider selectors fixed while PreparedSource replaces projection and batch size.
struct HarvestAccessor {
    harvester: OaiHarvester,
    selection: OaiHarvestRequest,
    max_records: Option<usize>,
}

#[async_trait]
impl Accessor for HarvestAccessor {
    type Request = ReadRequest;
    type Reader = dyn RecordReader;

    fn capability(&self) -> RecordCapability {
        self.harvester.capability()
    }

    async fn open(&self, read: ReadRequest) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        let mut selection = self.selection.clone();
        selection.read = read;
        let reader = self.harvester.open(selection).await?;
        let Some(remaining) = self.max_records else {
            return Ok(reader);
        };
        let metadata = reader.metadata().clone();
        let capability = reader.capability();
        let batches =
            stream::try_unfold((reader, remaining), |(mut reader, remaining)| async move {
                if remaining == 0 {
                    return Ok(None);
                }
                let Some(batch) = reader.next().await? else {
                    return Ok(None);
                };
                let count = batch.num_rows().min(remaining);
                Ok(Some((batch.slice(0, count), (reader, remaining - count))))
            });
        Ok(Box::new(StreamReader::records_with_capability(
            batches, metadata, capability,
        )))
    }
}
