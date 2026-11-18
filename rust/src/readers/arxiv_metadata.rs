use open_arrow_access::service::arxiv_metadata::ArxivMetadata;

pub type ArxivMetadataOptions = crate::oai::OaiOptions;

crate::ffi::reader::export_reader!(duckport_arxiv_metadata_open, reader_options: ArxivMetadataOptions;
    |locator, storage_options, scan_options| async move {
        let selection = unsafe { reader_options.read() }?;
        crate::oai::open::<ArxivMetadata>(locator, storage_options, scan_options, selection).await
    }
);
