#pragma once

#include "duckdb.hpp"
#include "duckport.h"

namespace duckdb {
string ReadSourceString(const Value &value);
void RegisterStorageOptions(TypedKwargs &options);

class ReaderOptions {
  public:
	explicit ReaderOptions(const TableFunctionBindInput &input);
	ReaderOptions(const ReaderOptions &) = delete;
	ReaderOptions &operator=(const ReaderOptions &) = delete;
	::StorageOptions storage{};
	::ScanOptions scan{};

  private:
	string endpoint;
	string region;
	string access_key_id;
	string secret_access_key;
	string session_token;
	string token;
	string revision;
	string record_path;
	string format;
	string path;
	string config;
	string split;
};
} // namespace duckdb
