use crate::{source::DuckportSource as AsyncSource, storage, utils::optional_string, validation};
use open_arrow_access::{
    ReadRequest,
    service::{
        pmc::{PmcDatasetAccessor, PmcFormat, PmcInventoryShardAccessor, PmcInventoryShardRequest},
        pmc_inventory::PmcInventoryAccessor,
    },
    storage::OpenDalAccessor,
};
use std::ffi::c_char;

#[repr(C)]
pub struct PmcOptions {
    pub content_format: *const c_char,
}
#[repr(C)]
pub struct PmcShardOptions {
    pub manifest: *const c_char,
    pub shard: *const c_char,
    pub content_format: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_pmc_open, reader_options: PmcOptions;
    |locator, storage_options, scan_options| async move {
        let format = unsafe { optional_string(reader_options.content_format) }?
            .map(|value| value.parse::<PmcFormat>()).transpose()?.unwrap_or_default();
        let (operator, path) = if locator.ends_with('/') {
            (storage::directory_operator(&locator, &storage_options)?, "/".to_owned())
        } else {
            storage::operator(&locator, &storage_options)?
        };
        AsyncSource::plannable(
            PmcDatasetAccessor::new(OpenDalAccessor::new(operator)).with_format(format),
            scan_options.request(ReadRequest::new(path)),
        ).await
    }
);

crate::ffi::reader::export_reader!(duckport_pmc_shard_open, reader_options: PmcShardOptions;
    |locator, storage_options, scan_options| async move {
        let manifest = unsafe { optional_string(reader_options.manifest) }?;
        let shard = unsafe { optional_string(reader_options.shard) }?;
        let manifest = validation::required(manifest.as_deref(), "manifest is required")?;
        let shard = validation::required(shard.as_deref(), "shard is required")?;
        let format = unsafe { optional_string(reader_options.content_format) }?
            .map(|value| value.parse::<PmcFormat>()).transpose()?.unwrap_or_default();
        let operator = storage::directory_operator(&locator, &storage_options)?;
        AsyncSource::plannable(
            PmcInventoryShardAccessor::new(OpenDalAccessor::new(operator)),
            scan_options.request(PmcInventoryShardRequest::new(manifest, shard)),
        ).await.map(|source| match format {
            PmcFormat::Pdf => source,
            PmcFormat::Xml => source.with_content_url_source_column("xml_url"),
            PmcFormat::Txt => source.with_content_url_source_column("text_url"),
        })
    }
);

crate::ffi::reader::export_reader!(duckport_pmc_inventory_open;
    |locator, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        AsyncSource::plannable(
            PmcInventoryAccessor::new(OpenDalAccessor::new(operator)),
            scan_options.request(ReadRequest::new(path)),
        ).await
    }
);
