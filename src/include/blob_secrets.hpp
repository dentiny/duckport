#pragma once
#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"
#include "security_options.hpp"
namespace duckdb {
class ClientContext;
struct BlobSecret {
	string scope;
	string type;
	SecurityParameters security;
	DuckportStoragePolicy policy;
};
vector<BlobSecret> CaptureBlobSecrets(ClientContext &context);
} // namespace duckdb
