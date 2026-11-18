#pragma once

#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"
#include "security_options.hpp"
#include "blob_secrets.hpp"

namespace duckdb {
class ClientContext;
class ExtensionLoader;

void RegisterStorageSettings(ExtensionLoader &loader);
DuckportStoragePolicy ReadStoragePolicy(ClientContext &context);

class StorageParameters {
  public:
	StorageParameters(ClientContext &context, const string &locator, const string &secret_type);
	StorageParameters(const StorageParameters &) = delete;
	StorageParameters &operator=(const StorageParameters &) = delete;
	DuckportStorageOptions config{};

  private:
	string endpoint;
	string region;
    SecurityParameters security;
    vector<DuckportHttpHeader> headers;
	// Keep the snapshot and its borrowed C ABI views alive for the open call.
	vector<BlobSecret> http_secrets;
	vector<vector<DuckportHttpHeader>> http_secret_headers;
	vector<DuckportHttpSecret> http_secret_views;
};
} // namespace duckdb
