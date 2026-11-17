#pragma once
#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"
namespace duckdb {
class ClientContext;
struct BlobSecret {
	string scope;
	string type;
	string token;
	vector<string> header_names;
	vector<string> header_values;
	DuckportStoragePolicy policy;
};
vector<BlobSecret> CaptureBlobSecrets(ClientContext &context);
} // namespace duckdb
