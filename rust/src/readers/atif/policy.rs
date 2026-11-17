//! ATIF field storage hints, independent of each content part variant.

pub(crate) const SOURCE_TYPE: &str = "duckport:source_type";
pub(crate) const STORAGE_POLICY: &str = "duckport:storage_policy";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StoragePolicy {
    Text,
    Blob,
}

impl StoragePolicy {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Blob => "blob",
        }
    }
}
