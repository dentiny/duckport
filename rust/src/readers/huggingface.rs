use crate::{source::DuckportSource as AsyncSource, utils::optional_string};
use open_arrow_access::service::huggingface::*;
use std::ffi::c_char;

#[repr(C)]
pub struct HuggingFaceDatasetOptions {
    pub revision: *const c_char,
    pub config: *const c_char,
    pub split: *const c_char,
    pub allow_partial: i32,
}

crate::ffi::reader::export_reader!(duckport_huggingface_open, reader_options: HuggingFaceDatasetOptions;
    |locator, storage_options, scan_options| async move {
        crate::provider_auth::reject_http_headers(
            storage_options.security.http()?,
            "Hugging Face",
        )?;
        let revision = unsafe { optional_string(reader_options.revision) }?;
        let config = unsafe { optional_string(reader_options.config) }?;
        let split = unsafe { optional_string(reader_options.split) }?;
        let allow_partial = crate::utils::boolean("allow_partial", reader_options.allow_partial)?;

        let mut request = if locator.contains("://") {
            crate::validation::validate_huggingface_url_options(
                revision.as_deref(),
                config.as_deref(),
                split.as_deref(),
            )?;
            crate::utils::huggingface_request(&locator)?
        } else {
            let mut request = HuggingFaceDatasetRequest::new(
                &locator,
                config.unwrap_or_else(|| "default".into()),
                split.unwrap_or_else(|| "train".into()),
            );
            if let Some(revision) = revision {
                request = request.with_revision(revision);
            }
            request
        };
        if allow_partial {
            request = request.with_partial_exports();
        }

        let mut settings = HuggingFaceOptions::default();
        if let Some(endpoint) = storage_options.endpoint {
            settings = settings.with_endpoint(endpoint);
        }
        if let Some(token) = storage_options
            .security
            .http()?
            .and_then(|credentials| credentials.token())
        {
            settings = settings.with_token(token.to_owned());
        }
        let service = HuggingFaceService::new(settings)
            .await?
            .with_http_config(storage_options.policy.config()?)?;
        AsyncSource::plannable(service, scan_options.request(request)).await
    }
);
