#include "duckdb/common/string_util.hpp"
#include "duckdb/main/client_context.hpp"
#include "duckdb/main/secret/secret.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
const char *OptionalString(const string &value) { return value.empty() ? nullptr : value.c_str(); }
} // namespace

StorageParameters::StorageParameters(ClientContext &context, const string &locator, const string &secret_type) {
	if (secret_type == "huggingface") {
		const auto scope = StringUtil::StartsWith(locator, "hf://") ? locator : StringUtil::Format("hf://datasets/%s", locator);
		KeyValueSecretReader reader(*context.db, "huggingface", scope);
		reader.TryGetSecretKey("token", token);
	} else if (StringUtil::StartsWith(locator, "s3://")) {
		KeyValueSecretReader reader(*context.db, "s3", locator);
		reader.TryGetSecretKey("key_id", access_key_id);
		reader.TryGetSecretKey("secret", secret_access_key);
		reader.TryGetSecretKey("session_token", session_token);
		reader.TryGetSecretKey("endpoint", endpoint);
		reader.TryGetSecretKey("region", region);
		bool use_ssl = true;
		reader.TryGetSecretKey("use_ssl", use_ssl);
		if (!endpoint.empty() && endpoint.find("://") == string::npos) {
			endpoint = StringUtil::Format("%s://%s", use_ssl ? "https" : "http", endpoint);
		}
		string url_style;
		reader.TryGetSecretKey("url_style", url_style);
		if (!url_style.empty() && url_style != "path" && url_style != "vhost") {
			throw InvalidConfigurationException("S3 secret url_style must be either 'path' or 'vhost'");
		}
		config.virtual_host_style = url_style == "vhost" ? 1 : 0;
		config.allow_anonymous = access_key_id.empty() && secret_access_key.empty() && session_token.empty() ? 1 : 0;
	} else if (StringUtil::StartsWith(locator, "http://") || StringUtil::StartsWith(locator, "https://")) {
		KeyValueSecretReader reader(*context.db, "http", locator);
		reader.TryGetSecretKey("bearer_token", token);
	}
	config.endpoint = OptionalString(endpoint);
	config.region = OptionalString(region);
	config.access_key_id = OptionalString(access_key_id);
	config.secret_access_key = OptionalString(secret_access_key);
	config.session_token = OptionalString(session_token);
	config.token = OptionalString(token);
}
} // namespace duckdb
