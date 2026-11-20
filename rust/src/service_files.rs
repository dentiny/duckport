//! SQL attachment selection; membership and decoding belong to open-arrow-access.
use crate::{error::Result, metrics::ReadMetrics, source::DuckportSource, utils};
use open_arrow_access::{AttachmentAccessor, AttachmentOptions, ByteAccessor, ReadRequest};
use std::ffi::c_char;

/// Owned selection shared by provider adapters without changing their C ABI.
pub(crate) struct SelectedAttachment {
    pub(crate) file_name: String,
    pub(crate) options: AttachmentOptions,
}

/// Read the common attachment fields from a provider's borrowed FFI options.
pub(crate) trait AttachmentSelection {
    /// # Safety
    /// Non-null attachment strings must be live NUL-terminated UTF-8 for this call.
    unsafe fn read_attachment(&self) -> Result<SelectedAttachment>;
}

// Keep provider structs and their C ABI layouts unchanged. All implementations
// delegate field validation and owned conversion to SelectedAttachment.
macro_rules! impl_attachment_selection {
    ($options:ty) => {
        impl $crate::service_files::AttachmentSelection for $options {
            unsafe fn read_attachment(
                &self,
            ) -> $crate::error::Result<$crate::service_files::SelectedAttachment> {
                unsafe {
                    $crate::service_files::SelectedAttachment::read(
                        self.file_name,
                        self.format,
                        self.record_path,
                    )
                }
            }
        }
    };
}
pub(crate) use impl_attachment_selection;

impl SelectedAttachment {
    /// # Safety
    /// Non-null pointers must reference live NUL-terminated UTF-8 strings.
    pub(crate) unsafe fn read(
        file_name: *const c_char,
        format: *const c_char,
        record_path: *const c_char,
    ) -> Result<Self> {
        let file_name = crate::validation::nonempty(
            unsafe { utils::optional_string(file_name) }?,
            "file_name is required",
        )?;
        let format = unsafe { utils::optional_string(format) }?;
        let record_path = unsafe { utils::optional_string(record_path) }?;
        let options = utils::attachment_options(
            format.as_deref().unwrap_or("binary"),
            record_path.as_deref(),
        )?;
        Ok(Self { file_name, options })
    }

    pub(crate) async fn open<A: ByteAccessor + 'static>(
        self,
        accessor: A,
        options: crate::options::ScanOptions,
        metrics: ReadMetrics,
    ) -> Result<DuckportSource> {
        DuckportSource::new(
            AttachmentAccessor::new(
                crate::metrics::ObservedBytes::new(accessor, metrics),
                self.options,
            )?,
            options.request(ReadRequest::new(self.file_name)),
        )
        .await
    }
}
