use crate::{source::DuckportSource as AsyncSource, storage};
use open_arrow_access::{
    AccessorExt, ReadRequest,
    decompression::AutoDecompressionLayer,
    service::{pmc::PmcAccessor, pmc_inventory::PmcInventoryAccessor},
    storage::OpenDalAccessor,
};

crate::ffi::reader::export_reader!(duckport_pmc_open;
    |locator, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let storage = OpenDalAccessor::new(operator).layer(AutoDecompressionLayer::new());
        AsyncSource::new(
            PmcAccessor::with_default_decoder(storage),
            ReadRequest::new(path).with_options(scan_options),
        )
        .await
    }
);

crate::ffi::reader::export_reader!(duckport_pmc_inventory_open;
    |locator, storage_options, scan_options| async move {
        let (operator, path) = storage::operator(&locator, &storage_options)?;
        let storage = OpenDalAccessor::new(operator);
        AsyncSource::new(
            PmcInventoryAccessor::new(storage),
            ReadRequest::new(path).with_options(scan_options),
        )
        .await
    }
);
