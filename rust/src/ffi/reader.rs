//! Declare reader C entry points while keeping provider logic at the call site.
macro_rules! export_reader {
    ($name:ident $(, $reader:ident : $options:ty)?;
        |$locator:ident, $storage:ident, $scan:ident| $open:expr
    ) => {
        /// # Safety
        /// Inputs are borrowed for this call. Strings must be NUL-terminated UTF-8,
        /// option structs must be live, and output must point to writable source-pointer storage.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            locator: *const std::ffi::c_char,
            storage_options: &$crate::options::DuckportStorageOptions,
            scan_options: &$crate::options::DuckportScanOptions,
            $($reader: &$options,)?
            output: *mut *mut $crate::ffi::source::DuckportSource,
        ) -> *mut std::ffi::c_char {
            let open = |$locator: String, $storage: $crate::options::StorageConfig, $scan: $crate::options::ScanOptions| $open;
            unsafe {
                $crate::ffi::source::open_reader(
                    locator, storage_options, scan_options, output, open,
                )
            }
        }
    };
}

pub(crate) use export_reader;
