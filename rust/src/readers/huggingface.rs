use crate::{
    ffi::{DuckportSource, open_reader},
    options::{ScanOptions, StorageOptions},
    source::DuckportSource as AsyncSource,
};
use open_arrow_access::{
    Accessor, ReadRequest, RecordCapability, RecordReader, async_trait, service::huggingface::*,
};
use std::ffi::c_char;

struct Dataset {
    service: HuggingFaceService,
    request: HuggingFaceDatasetRequest,
}
#[async_trait]
impl Accessor for Dataset {
    type Request = ReadRequest;
    type Reader = dyn RecordReader;
    fn capability(&self) -> RecordCapability {
        self.service.capability()
    }
    async fn open(&self, request: ReadRequest) -> open_arrow_access::Result<Box<dyn RecordReader>> {
        self.service
            .dataset(self.request.clone().with_options(request.options))
            .await
    }
}

/// # Safety
/// Locator and option JSON are NUL-terminated UTF-8. Option structs are valid for
/// this call; output points to writable source-pointer storage. No inputs are retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_huggingface_open(
    locator: *const c_char,
    storage_options: *const StorageOptions,
    scan_options: *const ScanOptions,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_reader(
            locator,
            storage_options,
            scan_options,
            output,
            |locator, mut storage_options, mut options, read_options| async move {
                let mut settings = HuggingFaceOptions::default();
                if let Some(endpoint) = storage_options.remove("endpoint") {
                    settings = settings.with_endpoint(endpoint);
                }
                if let Some(token) = storage_options.remove("token") {
                    settings = settings.with_token(token);
                }
                let service = HuggingFaceService::new(settings).await?;
                let mut request = HuggingFaceDatasetRequest::new(
                    &locator,
                    options.remove("config").unwrap_or_else(|| "default".into()),
                    options.remove("split").unwrap_or_else(|| "train".into()),
                );
                if let Some(revision) = storage_options.remove("revision") {
                    request = request.with_revision(revision);
                }
                if options
                    .remove("allow_partial")
                    .map(|value| value.parse())
                    .transpose()?
                    .unwrap_or(false)
                {
                    request = request.with_partial_exports();
                }
                AsyncSource::new(
                    Dataset { service, request },
                    ReadRequest::new(locator).with_options(read_options),
                )
                .await
            },
        )
    }
}
