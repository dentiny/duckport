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
	DuckportStorageOptions config {};

private:
	string endpoint;
	string region;
	unique_ptr<SecurityFfiView> security_view;
	unique_ptr<SourceSecretsFfiView> http_sources_view;
};
} // namespace duckdb
