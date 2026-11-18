#pragma once
#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"

namespace duckdb {
class KeyValueSecret;
class KeyValueSecretReader;

struct HttpHeader {
	string name;
	string value;
};

// Owned authentication material, independent of the C ABI representation.
struct SecurityParameters {
	string access_key_id, secret_access_key, session_token, token;
	vector<HttpHeader> headers;
	bool has_token = false;
	bool allow_anonymous = false;
};

SecurityParameters ReadHttpSecurity(const KeyValueSecret &secret);
SecurityParameters ReadHttpSecurity(KeyValueSecretReader &reader);

// Owns both strings and the C ABI header array. Pointer-bearing views cannot be
// copied/moved or obtained from a temporary. Keep this object through FFI open.
class SecurityFfiView {
public:
	explicit SecurityFfiView(SecurityParameters parameters) : parameters(std::move(parameters)) {
		for (const auto &header : this->parameters.headers) {
			headers.push_back({header.name.c_str(), header.value.c_str()});
		}
		config.access_key_id = OptionalString(this->parameters.access_key_id);
		config.secret_access_key = OptionalString(this->parameters.secret_access_key);
		config.session_token = OptionalString(this->parameters.session_token);
		config.token = this->parameters.has_token ? this->parameters.token.c_str() : nullptr;
		config.http_headers = headers.data();
		config.http_header_count = headers.size();
		config.allow_anonymous = this->parameters.allow_anonymous;
	}

	SecurityFfiView(const SecurityFfiView &) = delete;
	SecurityFfiView &operator=(const SecurityFfiView &) = delete;
	SecurityFfiView(SecurityFfiView &&) = delete;
	SecurityFfiView &operator=(SecurityFfiView &&) = delete;

	const DuckportSecurityOptions &Get() const & {
		return config;
	}
	const DuckportSecurityOptions &Get() const && = delete;

private:
	static const char *OptionalString(const string &value) {
		return value.empty() ? nullptr : value.c_str();
	}
	SecurityParameters parameters;
	vector<DuckportHttpHeader> headers;
	DuckportSecurityOptions config {};
};
} // namespace duckdb
