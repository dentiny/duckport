use open_arrow_access::service::oapen::Oapen;

pub type OapenOptions = crate::oai::OaiOptions;

crate::ffi::reader::export_reader!(duckport_oapen_open, reader_options: OapenOptions;
    |locator, storage_options, scan_options| async move {
        let selection = unsafe { reader_options.read() }?;
        crate::oai::open::<Oapen>(locator, storage_options, scan_options, selection).await
    }
);
