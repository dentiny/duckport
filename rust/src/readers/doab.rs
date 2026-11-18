use open_arrow_access::service::doab::Doab;

pub type DoabOptions = crate::oai::OaiOptions;

crate::ffi::reader::export_reader!(duckport_doab_open, reader_options: DoabOptions;
    |locator, storage_options, scan_options| async move {
        let selection = unsafe { reader_options.read() }?;
        crate::oai::open::<Doab>(locator, storage_options, scan_options, selection).await
    }
);
