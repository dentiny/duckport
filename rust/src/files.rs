//! File selection at bind time; decoding and schema checks stay in upstream accessors.
use crate::{error::Result, storage, utils, validation};
use open_arrow_access::{
    ListRequest, decompression::CompressionFactory, list_files, storage::OpenDalAccessor,
};
use url::Url;

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

/// Keep parsing and URL escaping in one place; paths are decoded only for glob matching.
fn parse(locator: &str) -> Result<Url> {
    if locator.is_empty() {
        return Err("file selection cannot be empty".into());
    }
    if storage::is_local(locator) {
        return Url::from_file_path(utils::local_path(locator)?)
            .map_err(|_| "invalid local path".into());
    }
    let url = Url::parse(locator)?;
    validation::validate_uri(&url)?;
    validation::validate_storage_scheme(url.scheme())?;
    if url.scheme() == "s3" {
        // Inspect the original path: URL parsing normalizes dot segments, which
        // could otherwise move discovery outside the selected credential scope.
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
    Ok(url)
}

// Decode literal URI text separately from unescaped glob operators. Escaped
// metacharacters must stay literal even inside a pattern such as data%5B1%5D*.json.
fn decode_uri_glob(path: &str) -> Result<String> {
    let mut pattern = String::new();
    for part in path.split_inclusive(['*', '?', '[', ']']) {
        let end = part.len() - usize::from(part.ends_with(['*', '?', '[', ']']));
        let literal = percent_encoding::percent_decode_str(&part[..end]).decode_utf8()?;
        pattern.push_str(&glob::Pattern::escape(&literal));
        pattern.push_str(&part[end..]);
    }
    Ok(pattern)
}

/// Split a glob into its literal directory URL and relative matching pattern.
fn glob_root(mut url: Url, preserve_escapes: bool) -> Result<(Url, Option<glob::Pattern>)> {
    // URL parsing escapes some local-path glob characters itself. Only explicit
    // URIs distinguish percent-encoded literals from raw glob syntax.
    let path = if preserve_escapes {
        url.path().to_owned()
    } else {
        percent_encoding::percent_decode_str(url.path())
            .decode_utf8()?
            .into_owned()
    };
    let Some(first) = path.find(['*', '?', '[']) else {
        return Ok((url, None));
    };
    let split = path[..first].rfind('/').map_or(0, |p| p + 1);
    let (root, pattern) = if preserve_escapes {
        (
            percent_encoding::percent_decode_str(&path[..split])
                .decode_utf8()?
                .into_owned(),
            decode_uri_glob(&path[split..])?,
        )
    } else {
        (path[..split].to_owned(), path[split..].to_owned())
    };
    let pattern = glob::Pattern::new(&pattern)?;
    url.path_segments_mut()
        .map_err(|_| "invalid directory URI")?
        .clear()
        .extend(root.strip_prefix('/').unwrap_or(&root).split('/'));
    Ok((url, Some(pattern)))
}

pub(crate) async fn resolve(
    locator: &str,
    format: &str,
    config: &crate::options::StorageConfig,
) -> Result<Vec<String>> {
    let mut base = parse(locator)?;
    let metadata = if base.scheme() == "file" {
        tokio::fs::metadata(utils::local_path(locator)?).await.ok()
    } else {
        None
    };
    // Existing local paths take precedence over glob syntax in their names.
    if metadata.as_ref().is_some_and(|m| m.is_file()) {
        return Ok(vec![locator.to_owned()]);
    }
    let directory = base.path().ends_with('/') || metadata.is_some_and(|m| m.is_dir());
    let pattern = if directory {
        if !base.path().ends_with('/') {
            base.path_segments_mut()
                .map_err(|_| "invalid directory URI")?
                .push("");
        }
        None
    } else {
        let (root, pattern) = glob_root(base, locator.contains("://"))?;
        let Some(pattern) = pattern else {
            return Ok(vec![locator.to_owned()]);
        };
        base = root;
        Some(pattern)
    };
    let operator = storage::directory_operator(base.as_str(), config)?;
    let match_options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    let compression = CompressionFactory::default();
    let objects = list_files(
        &OpenDalAccessor::new(operator),
        ListRequest::new("").with_recursive(
            pattern
                .as_ref()
                .is_none_or(|p| p.as_str().contains('/') || p.as_str().contains("**")),
        ),
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
            url.path_segments_mut()
                .map_err(|_| "invalid directory URI")?
                .pop_if_empty()
                .extend(object.locator().split('/'));
            Ok(url.to_string())
        })
        .collect()
}

/// URI used by the SQL layer to select discovery credentials before I/O.
pub(crate) fn credential_scope(locator: &str) -> Result<String> {
    let url = parse(locator)?;
    if url.scheme() == "file" {
        // Local paths do not need scoped remote credentials or glob parsing.
        return Ok(locator.to_owned());
    }
    Ok(glob_root(url, locator.contains("://"))?.0.to_string())
}
