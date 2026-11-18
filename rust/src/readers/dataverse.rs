use crate::utils::optional_string;
use open_arrow_access::service::dataverse::DataverseService;
use std::ffi::c_char;

#[repr(C)]
pub struct DataverseOptions {
    pub file_name: *const c_char,
    pub format: *const c_char,
    pub record_path: *const c_char,
    pub endpoint: *const c_char,
    pub version: *const c_char,
}

crate::ffi::reader::export_reader!(duckport_dataverse_open, reader_options: DataverseOptions;
    |locator, storage_options, scan_options| async move {
        let attachment = unsafe {
            crate::service_files::SelectedAttachment::read(
                reader_options.file_name,
                reader_options.format,
                reader_options.record_path,
            )
        }?;
        let endpoint = unsafe { optional_string(reader_options.endpoint) }?
            .unwrap_or_else(|| "https://dataverse.harvard.edu".into());
        let version = crate::validation::nonempty(
            unsafe { optional_string(reader_options.version) }?,
            "version is required",
        )?;
        let service =
            DataverseService::new(&endpoint)?.with_http_config(storage_options.policy.config()?)?;
        let anonymous = crate::security::HttpCredentials::default();
        let credentials = storage_options.security.http()?.unwrap_or(&anonymous);
        let headers = credentials.headers();
        if headers.keys().any(|name| name != "x-dataverse-key") {
            return Err("Dataverse supports only X-Dataverse-key in EXTRA_HTTP_HEADERS".into());
        }
        let api_key = headers.get("x-dataverse-key");
        let service = match (credentials.token(), api_key) {
            (Some(_), Some(_)) => {
                return Err("choose BEARER_TOKEN or X-Dataverse-key for Dataverse".into());
            }
            (Some(token), None) => service.with_bearer_token(token)?,
            (None, Some(key)) => service.with_token(key.to_str()?)?,
            (None, None) => service,
        };
        let files = service.dataset(&locator, &version).await?;
        attachment
            .open(files, scan_options, storage_options.metrics.clone())
            .await
    }
);
