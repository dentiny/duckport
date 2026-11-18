//! Explicit provider token mappings and shared validation of HTTP credential headers.
use crate::{error::Result, options::StorageConfig, security::HttpCredentials, validation};
use open_arrow_access::service::common::{ApiKey, Connection};

#[derive(Clone, Copy)]
pub(crate) enum HttpAuth {
    Bearer,
    FigshareToken,
    ApiKeyHeader(&'static str),
    ApiKeyQuery(&'static str),
}

/// Construct the primary provider connection using DuckDB's storage policy and secret.
/// Secondary connections must be built separately without these credentials.
pub(crate) fn connection(
    endpoint: &str,
    config: &StorageConfig,
    auth: HttpAuth,
) -> Result<Connection> {
    apply(
        Connection::new(endpoint)?.with_http_config(config.policy.config()?)?,
        config.security.http()?,
        auth,
    )
}

/// Apply credentials only to the provider's primary connection. The upstream
/// connection keeps headers/query credentials on its origin during downloads.
fn apply(
    mut connection: Connection,
    security: Option<&HttpCredentials>,
    auth: HttpAuth,
) -> Result<Connection> {
    let anonymous = HttpCredentials::default();
    let security = security.unwrap_or(&anonymous);
    let headers = security.headers();
    let token = security.token();
    let token_header = match auth {
        HttpAuth::Bearer | HttpAuth::FigshareToken => Some("authorization"),
        HttpAuth::ApiKeyHeader(name) => Some(name),
        HttpAuth::ApiKeyQuery(_) => None,
    };
    if token.is_some() && token_header.is_some_and(|name| headers.contains_key(name)) {
        return Err("BEARER_TOKEN conflicts with the provider authentication header".into());
    }
    match auth {
        HttpAuth::Bearer | HttpAuth::FigshareToken => {
            if let Some(token) = token {
                let scheme = if matches!(auth, HttpAuth::Bearer) {
                    "Bearer"
                } else {
                    "token"
                };
                connection =
                    connection.with_header("authorization", &format!("{scheme} {token}"))?;
            }
        }
        HttpAuth::ApiKeyHeader(name) => {
            // A custom header may supply the native API key directly. Never
            // overwrite it silently with a differently configured token.
            let key = token.or_else(|| headers.get(name).and_then(|value| value.to_str().ok()));
            let key = validation::nonempty(
                key,
                "a scoped HTTP API key secret is required for this service",
            )?;
            connection = connection.with_api_key(ApiKey::Header(name), key)?;
        }
        HttpAuth::ApiKeyQuery(name) => {
            let key = validation::nonempty(
                token,
                "a scoped HTTP bearer_token secret is required for this service",
            )?;
            connection = connection.with_api_key(ApiKey::Query(name), key)?;
        }
    }
    for (name, value) in headers {
        connection = connection.with_header(name.as_str(), value.to_str()?)?;
    }
    Ok(connection)
}

pub(crate) fn reject_http_headers(
    security: Option<&HttpCredentials>,
    adapter: &'static str,
) -> Result<()> {
    if security.is_some_and(|credentials| !credentials.headers().is_empty()) {
        return Err(format!("{adapter} does not support EXTRA_HTTP_HEADERS").into());
    }
    Ok(())
}
