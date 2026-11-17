#pragma once

#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"

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
	string access_key_id;
	string secret_access_key;
	string session_token;
	string token;
    vector<string> header_names;
    vector<string> header_values;
    vector<DuckportHttpHeader> headers;
};
} // namespace duckdb
