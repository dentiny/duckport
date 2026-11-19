use crate::source::DuckportSource as AsyncSource;
use open_arrow_access::{
    Accessor, ByteReadRequest,
    decompression::zip::ZipLimits,
    service::meca::{MecaArchive, MecaReadRequest},
    storage::OpenDalAccessor,
};
crate::ffi::reader::export_reader!(duckport_meca_open;
    |locator, storage_options, scan_options| async move {
        let (operator, path) = crate::storage::operator(&locator, &storage_options)?;
        let input = OpenDalAccessor::new(operator)
            .open(ByteReadRequest::new(path).with_options(scan_options.bytes))
            .await?;
        let records = MecaArchive::open(input, ZipLimits::default())
            .await?
            .records();
        AsyncSource::new(
            records,
            scan_options.request(MecaReadRequest::default()),
        )
        .await
    }
);
