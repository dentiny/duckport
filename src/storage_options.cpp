#include "duckdb/common/string_util.hpp"
#include "duckdb/main/client_context.hpp"
#include "duckdb/main/client_data.hpp"
#include "duckdb/common/file_opener.hpp"
#include "duckdb/main/secret/secret.hpp"
#include "storage_options.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/identifier.hpp"
#include "duckdb/main/config.hpp"
#include "duckdb/main/extension/extension_loader.hpp"

#include <cmath>
#include <limits>
#include <string_view>

namespace duckdb {
namespace {
const char *OptionalString(const string &value) {
	return value.empty() ? nullptr : value.c_str();
}

constexpr std::string_view STORAGE_SETTING_PREFIX = "conversion_storage_";

template <bool ALLOW_ZERO>
void SetStorageInteger(ClientContext &, SetScope, Value &value) {
	if (value.IsNull() || value.GetValue<int64_t>() < (ALLOW_ZERO ? 0 : 1)) {
		throw InvalidInputException(ALLOW_ZERO ? "Storage retry_max_times must be non-negative"
		                                       : "Storage timeout and retry delay settings must be positive");
	}
}

void SetStorageRetryFactor(ClientContext &, SetScope, Value &value) {
	auto factor = value.IsNull() ? 0 : value.GetValue<double>();
	if (!std::isfinite(factor) || factor < 1 || factor > std::numeric_limits<float>::max()) {
		throw InvalidInputException("Storage retry_factor must be finite and between 1 and f32::MAX");
	}
}

template <typename T>
T ReadStorageSetting(ClientContext &context, const char *suffix) {
	auto name = string(STORAGE_SETTING_PREFIX) + suffix;
	Value value;
	if (!context.TryGetCurrentSetting(Identifier(name), value)) {
		throw InternalException("Missing storage setting: %s", name);
	}
	return value.GetValue<T>();
}

} // namespace

void RegisterStorageSettings(ExtensionLoader &loader) {
	auto &config = DBConfig::GetConfig(loader.GetDatabaseInstance());
	const auto defaults = duckport_default_storage_policy();
	auto register_integer = [&](const char *name, const char *description, uint64_t value, bool allow_zero = false) {
		config.AddExtensionOption(Identifier(string(STORAGE_SETTING_PREFIX) + name), description, LogicalType::BIGINT,
		                          Value::BIGINT(value),
		                          allow_zero ? SetStorageInteger<true> : SetStorageInteger<false>);
	};
	register_integer("timeout_ms", "Control operation timeout per attempt (ms)", defaults.timeout_ms);
	register_integer("io_timeout_ms", "IO operation and body method timeout per attempt (ms)", defaults.io_timeout_ms);
	register_integer("retry_max_times", "Maximum retries after the initial attempt (0 disables retries)",
	                 defaults.retry_max_times, true);
	register_integer("retry_min_delay_ms", "Initial exponential retry backoff (ms)", defaults.retry_min_delay_ms);
	register_integer("retry_max_delay_ms", "Maximum exponential retry backoff (ms)", defaults.retry_max_delay_ms);
	config.AddExtensionOption(Identifier(string(STORAGE_SETTING_PREFIX) + "retry_factor"),
	                          "Storage exponential retry backoff multiplier for reads and writes", LogicalType::DOUBLE,
	                          Value::DOUBLE(defaults.retry_factor), SetStorageRetryFactor);
}

// Readers and COPY writers share the current connection's policy.
DuckportStoragePolicy ReadStoragePolicy(ClientContext &context) {
	DuckportStoragePolicy result {
	    ReadStorageSetting<uint64_t>(context, "timeout_ms"),
	    ReadStorageSetting<uint64_t>(context, "io_timeout_ms"),
	    ReadStorageSetting<uint64_t>(context, "retry_max_times"),
	    ReadStorageSetting<uint64_t>(context, "retry_min_delay_ms"),
	    ReadStorageSetting<uint64_t>(context, "retry_max_delay_ms"),
	    ReadStorageSetting<double>(context, "retry_factor"),
	};
	if (result.retry_max_delay_ms < result.retry_min_delay_ms) {
		throw InvalidInputException("conversion_storage_retry_max_delay_ms must be at least retry_min_delay_ms");
	}
	return result;
}

StorageParameters::StorageParameters(ClientContext &context, const string &locator, const string &secret_type) {
	SecurityParameters security;
	config.policy = ReadStoragePolicy(context);
	auto &opener = *ClientData::Get(context).file_opener;
	FileOpenerInfo info {locator};
	if (secret_type == "huggingface") {
		const auto scope =
		    StringUtil::StartsWith(locator, "hf://") ? locator : StringUtil::Format("hf://datasets/%s", locator);
		info.file_path = scope;
		KeyValueSecretReader reader(opener, &info, "huggingface");
		security.has_token = reader.TryGetSecretKey("token", security.token);
	} else if (secret_type == "internet_archive") {
		KeyValueSecretReader reader(opener, &info, "s3");
		reader.TryGetSecretKey("key_id", security.access_key_id);
		reader.TryGetSecretKey("secret", security.secret_access_key);
	} else if (StringUtil::StartsWith(locator, "s3://")) {
		KeyValueSecretReader reader(opener, &info, "s3");
		reader.TryGetSecretKey("key_id", security.access_key_id);
		reader.TryGetSecretKey("secret", security.secret_access_key);
		reader.TryGetSecretKey("session_token", security.session_token);
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
		security.allow_anonymous =
		    security.access_key_id.empty() && security.secret_access_key.empty() && security.session_token.empty() ? 1
		                                                                                                           : 0;
	} else if (StringUtil::StartsWith(locator, "http://") || StringUtil::StartsWith(locator, "https://")) {
		// Preserve every HTTP scope for credential selection on each redirect hop.
		http_sources_view = make_uniq<SourceSecretsFfiView>(CaptureBlobSecrets(context, "http"));
		http_sources_view->BindHttp(config);
		KeyValueSecretReader reader(opener, &info, "http");
		security = ReadHttpSecurity(reader);
	}
	config.endpoint = OptionalString(endpoint);
	config.region = OptionalString(region);
	security_view = make_uniq<SecurityFfiView>(std::move(security));
	config.security = security_view->Get();
}
} // namespace duckdb
