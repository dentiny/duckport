mod metadata;

use crate::{source::DuckportSource as AsyncSource, utils::optional_string};
use open_arrow_access::{
    service::arxiv::{ArxivReadRequest, ArxivService},
    storage::OpenDalAccessor,
};
use std::ffi::c_char;
#[repr(C)]
pub struct ArxivOptions {
    pub kind: *const c_char,
    pub month: *const c_char,
    pub requester_pays: i32,
    pub metadata_uri: *const c_char,
    pub metadata_id_column: *const c_char,
    pub metadata_storage: *const crate::options::DuckportStorageOptions,
}

crate::ffi::reader::export_reader!(duckport_arxiv_open, reader_options: ArxivOptions;
    |locator, storage_options, scan_options| async move {
        let kind =
            unsafe { optional_string(reader_options.kind) }?.unwrap_or_else(|| "source".into());
        let month = unsafe { optional_string(reader_options.month) }?;
        let kind = crate::utils::arxiv_kind(&kind)?;
        let metadata_uri = unsafe { optional_string(reader_options.metadata_uri) }?;
        let id_column = unsafe { optional_string(reader_options.metadata_id_column) }?;
        crate::validation::validate_arxiv_metadata(kind, metadata_uri.as_deref(), id_column.as_deref())?;
        let month = month.map(|s| s.parse::<u16>()).transpose()?;
        let requester_pays =
            crate::utils::boolean("requester_pays", reader_options.requester_pays)?;
        let storage = OpenDalAccessor::new(crate::storage::directory_operator_with_requester_pays(
            &locator,
            &storage_options,
            requester_pays,
        )?);
        let records = ArxivService::new(storage).records();
        let mut request = scan_options.request(ArxivReadRequest::new(kind));
        if let Some(month) = month {
            request = request.with_month(month);
        }
        if let Some(uri) = metadata_uri {
            let config = crate::validation::required(
                unsafe { reader_options.metadata_storage.as_ref() },
                "metadata_uri requires its own storage configuration",
            )?;
            let mut config = unsafe { config.read() }?;
            config.metrics = storage_options.metrics.clone();
            let index = metadata::load(&uri, id_column.as_deref().unwrap_or("identifier"), &config).await?;
            AsyncSource::plannable(records.with_metadata(index), request).await
        } else {
            AsyncSource::plannable(records, request).await
        }
    }
);
