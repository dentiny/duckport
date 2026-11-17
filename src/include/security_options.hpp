#pragma once
#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"
namespace duckdb {
// Owned authentication material. Only its temporary C ABI view crosses the bridge.
struct SecurityParameters {
    string access_key_id, secret_access_key, session_token, token;
    vector<string> header_names, header_values;
    bool has_token = false;
    bool allow_anonymous = false;
    DuckportSecurityOptions ToFfi(vector<DuckportHttpHeader> &headers) const {
        headers.clear();
        for (idx_t i = 0; i < header_names.size(); i++) { headers.push_back({header_names[i].c_str(), header_values[i].c_str()}); }
        DuckportSecurityOptions result {};
        result.access_key_id = access_key_id.empty() ? nullptr : access_key_id.c_str();
        result.secret_access_key = secret_access_key.empty() ? nullptr : secret_access_key.c_str();
        result.session_token = session_token.empty() ? nullptr : session_token.c_str();
        result.token = has_token ? token.c_str() : nullptr;
        result.http_headers = headers.data(); result.http_header_count = headers.size();
        result.allow_anonymous = allow_anonymous;
        return result;
    }
};
} // namespace duckdb
