use std::ffi::c_char;

#[repr(C)]
pub struct StorageOptions {
    pub json: *const c_char,
}

#[repr(C)]
pub struct ScanOptions {
    pub batch_size: usize,
    pub json: *const c_char,
}
