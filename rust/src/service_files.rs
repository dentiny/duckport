//! SQL attachment selection; membership and decoding belong to open-arrow-access.
use crate::{error::Result, metrics::ReadMetrics, source::DuckportSource, utils};
use open_arrow_access::{
    AttachmentAccessor, AttachmentOptions, ByteAccessor, ReadOptions, ReadRequest,
};
use std::ffi::c_char;

/// Owned selection shared by provider adapters without changing their C ABI.
pub(crate) struct SelectedAttachment {
    file_name: String,
    options: AttachmentOptions,
}

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
        options: ReadOptions,
        metrics: ReadMetrics,
    ) -> Result<DuckportSource> {
        DuckportSource::new(
            AttachmentAccessor::new(
                crate::metrics::ObservedBytes::new(accessor, metrics),
                self.options,
            )?,
            ReadRequest::new(self.file_name).with_options(options),
        )
        .await
    }
}
