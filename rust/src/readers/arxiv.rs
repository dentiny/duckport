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
}

crate::ffi::reader::export_reader!(duckport_arxiv_open, reader_options: ArxivOptions;
    |locator, storage_options, scan_options| async move {
        let kind =
            unsafe { optional_string(reader_options.kind) }?.unwrap_or_else(|| "source".into());
        let month = unsafe { optional_string(reader_options.month) }?;
        let kind = crate::utils::arxiv_kind(&kind)?;
        let month = month.map(|s| s.parse::<u16>()).transpose()?;
        let requester_pays =
            crate::utils::boolean("requester_pays", reader_options.requester_pays)?;
        let storage = OpenDalAccessor::new(crate::storage::directory_operator_with_requester_pays(
            &locator,
            &storage_options,
            requester_pays,
        )?);
        let records = ArxivService::new(storage).records();
        let mut request = ArxivReadRequest::new(kind).with_options(scan_options);
        if let Some(month) = month {
            request = request.with_month(month);
        }
        AsyncSource::new(records, request).await
    }
);
