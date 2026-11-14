use crate::{
    ffi::{DuckportSource, open_reader},
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
/// Strings are NUL-terminated UTF-8; output points to writable source-pointer storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_huggingface_open(
    locator: *const c_char,
    options: *const c_char,
    batch_size: usize,
    output: *mut *mut DuckportSource,
) -> *mut c_char {
    unsafe {
        open_reader(
            locator,
            options,
            batch_size,
            output,
            |locator, mut options, read_options| async move {
                let mut settings = HuggingFaceOptions::default();
                if let Some(endpoint) = options.remove("endpoint") {
                    settings = settings.with_endpoint(endpoint);
                }
                if let Some(token) = options.remove("token") {
                    settings = settings.with_token(token);
                }
                let service = HuggingFaceService::new(settings).await?;
                let mut request = HuggingFaceDatasetRequest::new(
                    &locator,
                    options.remove("config").unwrap_or_else(|| "default".into()),
                    options.remove("split").unwrap_or_else(|| "train".into()),
                );
                if let Some(revision) = options.remove("revision") {
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
