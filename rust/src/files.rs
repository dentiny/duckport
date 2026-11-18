//! File selection at bind time; decoding and schema checks stay in upstream accessors.
use crate::{
    error::{Result, call, cleanup},
    ffi::common::RUNTIME,
    options::DuckportStorageOptions,
    storage, utils, validation,
};
use open_arrow_access::{
    ListRequest, decompression::CompressionFactory, list_files, storage::OpenDalAccessor,
};
use std::{
    ffi::{CString, c_char},
    ptr,
};
use url::Url;

pub struct DuckportFileList(Vec<CString>);

fn matches_format(path: &str, format: &str, compression: &CompressionFactory) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = if format == "parquet" {
        lower.as_str()
    } else {
        compression.strip_suffix(&lower)
    };
    match format {
        "json" | "jsonl" | "ndjson" => [".json", ".jsonl", ".ndjson"]
            .iter()
            .any(|s| name.ends_with(s)),
        other => name.ends_with(&format!(".{other}")),
    }
}

fn location(locator: &str) -> Result<(Url, String, bool)> {
    if locator.is_empty() {
        return Err("file selection cannot be empty".into());
    }
    let local = storage::is_local(locator);
    let (base, path) = if local {
        let path = utils::local_path(locator)?;
        (
            Url::from_file_path(&path).map_err(|_| "invalid local path")?,
            {
                let text = path.to_str().ok_or("path must be UTF-8")?;
                if cfg!(windows) {
                    text.replace('\\', "/")
                } else {
                    text.to_owned()
                }
            },
        )
    } else {
        let mut url = Url::parse(locator)?;
        validation::validate_uri(&url)?;
        validation::validate_storage_scheme(url.scheme())?;
        if url.scheme() == "s3" {
            // URL normalization must not move discovery outside the scope for
            // which DuckDB selected its credentials.
            let raw_path = locator
                .split_once("://")
                .and_then(|(_, rest)| rest.split_once('/'))
                .map_or("", |(_, path)| path);
            if percent_encoding::percent_decode_str(raw_path)
                .decode_utf8()?
                .split('/')
                .any(|part| matches!(part, "." | ".."))
            {
                return Err("S3 selections cannot contain dot path segments".into());
            }
        }
        let path = percent_encoding::percent_decode_str(url.path())
            .decode_utf8()?
            .into_owned();
        url.set_path("/");
        (url, path)
    };
    Ok((base, path, local))
}

fn discovery_root(
    mut base: Url,
    path: &str,
    wildcard: Option<usize>,
    local: bool,
) -> Result<(Url, Option<glob::Pattern>)> {
    let (root, pattern) = if let Some(first) = wildcard {
        let split = path[..first].rfind('/').map_or(0, |p| p + 1);
        (&path[..split], Some(glob::Pattern::new(&path[split..])?))
    } else {
        (path, None)
    };
    if local {
        base = Url::from_directory_path(root).map_err(|_| "invalid directory selection")?;
    } else {
        base.path_segments_mut()
            .map_err(|_| "invalid directory URI")?
            .clear()
            .extend(root.strip_prefix('/').unwrap_or(root).split('/'));
        if !base.path().ends_with('/') {
            base.path_segments_mut()
                .map_err(|_| "invalid directory URI")?
                .push("");
        }
    }
    Ok((base, pattern))
}

async fn resolve(
    locator: &str,
    format: &str,
    config: &crate::options::StorageConfig,
) -> Result<Vec<String>> {
    let (base, path, local) = location(locator)?;
    let wildcard = path.find(['*', '?', '[']);
    let directory =
        path.ends_with('/') || local && tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_dir());
    if wildcard.is_none() && !directory {
        return Ok(vec![locator.to_owned()]);
    }
    let (base, pattern) = discovery_root(base, &path, wildcard, local)?;
    let operator = storage::directory_operator(base.as_str(), config)?;
    let match_options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    let compression = CompressionFactory::default();
    let objects = list_files(
        &OpenDalAccessor::new(operator),
        ListRequest::new("").with_recursive(true),
        |relative| {
            Ok(!relative.is_empty()
                && !relative.ends_with('/')
                && match &pattern {
                    Some(pattern) => pattern.matches_with(relative, match_options),
                    None => matches_format(relative, format, &compression),
                })
        },
    )
    .await?;
    if objects.is_empty() {
        return Err("file selection matched no files".into());
    }
    objects
        .into_iter()
        .map(|object| {
            let mut url = base.clone();
            let root = percent_encoding::percent_decode_str(base.path())
                .decode_utf8()?
                .into_owned();
            let root = root.strip_prefix('/').unwrap_or(&root);
            let root = root.strip_suffix('/').unwrap_or(root);
            {
                let mut segments = url
                    .path_segments_mut()
                    .map_err(|_| "invalid directory URI")?;
                segments.clear();
                if !root.is_empty() {
                    segments.extend(root.split('/'));
                }
                segments.extend(object.locator().split('/'));
            }
            Ok(url.to_string())
        })
        .collect()
}

/// # Safety
/// Locator is borrowed and NUL-terminated; output is writable. The returned
/// one-element list owns the credential lookup URI until destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_discovery_root(
    locator: *const c_char,
    output: *mut *mut DuckportFileList,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null discovery root output")?;
        *output = ptr::null_mut();
        let locator = utils::string(locator)?;
        let (base, path, local) = location(&locator)?;
        let root = match path.find(['*', '?', '[']) {
            Some(first) => discovery_root(base, &path, Some(first), local)?
                .0
                .to_string(),
            None => locator,
        };
        *output = Box::into_raw(Box::new(DuckportFileList(vec![CString::new(root)?])));
        Ok(())
    })
}

/// # Safety
/// Strings/options are borrowed for this call; output is writable. Free its
/// returned handle once with duckport_file_list_destroy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_resolve_files(
    locator: *const c_char,
    format: *const c_char,
    storage: &DuckportStorageOptions,
    output: *mut *mut DuckportFileList,
) -> *mut c_char {
    call(|| unsafe {
        validation::non_null(output, "null file selection output")?;
        *output = ptr::null_mut();
        let files = RUNTIME.block_on(resolve(
            &utils::string(locator)?,
            &utils::string(format)?,
            &storage.read()?,
        ))?;
        let files = files
            .into_iter()
            .map(CString::new)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        *output = Box::into_raw(Box::new(DuckportFileList(files)));
        Ok(())
    })
}
/// # Safety
/// The handle is live and borrowed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_list_count(files: &DuckportFileList) -> usize {
    files.0.len()
}
/// # Safety
/// The handle is live and index is less than duckport_file_list_count. The
/// returned string is borrowed until the handle is destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_list_get(
    files: &DuckportFileList,
    index: usize,
) -> *const c_char {
    files.0[index].as_ptr()
}
/// # Safety
/// The handle is null or live, and freed only once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn duckport_file_list_destroy(files: *mut DuckportFileList) {
    cleanup(|| {
        if !files.is_null() {
            drop(unsafe { Box::from_raw(files) });
        }
    });
}
