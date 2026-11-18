#include "security_options.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/file_opener.hpp"
#include "duckdb/common/helper.hpp"
#include "duckdb/main/secret/secret.hpp"

namespace duckdb {
namespace {
template <typename ReadValue>
SecurityParameters ReadHttpSecurityValues(ReadValue read) {
	SecurityParameters result;
	Value token;
	if (read("bearer_token", token) && !token.IsNull()) {
		result.token = token.GetValue<string>();
		result.has_token = true;
	}
	Value headers;
	if (read("extra_http_headers", headers) && !headers.IsNull()) {
		for (const auto &item : MapValue::GetChildren(headers)) {
			const auto &pair = StructValue::GetChildren(item);
			if (pair[0].IsNull() || pair[1].IsNull()) {
				throw InvalidInputException("HTTP header names and values must not be NULL");
			}
			result.headers.push_back({pair[0].GetValue<string>(), pair[1].GetValue<string>()});
		}
	}
	return result;
}
} // namespace

SecurityParameters ReadHttpSecurity(const KeyValueSecret &secret) {
	return ReadHttpSecurityValues([&](const char *key, Value &value) { return secret.TryGetValue(key, value); });
}
SecurityParameters ReadHttpSecurity(KeyValueSecretReader &reader) {
	return ReadHttpSecurityValues([&](const char *key, Value &value) { return reader.TryGetSecretKey(key, value); });
}

} // namespace duckdb
