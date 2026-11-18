#pragma once

#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"

namespace duckdb {
struct FixtureRequest {
	string origin;
	string target;
	string authorization;
	string api_key;
	string headers;
	string request_payer;
	bool is_download;
	bool has_authorization;
	bool has_request_payer;
};

string ProviderFixtureCommand(const string &provider, const string &command);
vector<FixtureRequest> ProviderFixtureRequests(const string &provider);
} // namespace duckdb
