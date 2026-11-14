#pragma once

#include "duckdb/common/string.hpp"
#include "duckport.h"

namespace duckdb {
class ClientContext;

class StorageParameters {
  public:
	StorageParameters(ClientContext &context, const string &locator, const string &secret_type,
					  const char *file_path = nullptr);
	StorageParameters(const StorageParameters &) = delete;
	StorageParameters &operator=(const StorageParameters &) = delete;
	::StorageOptions config{};

  private:
	string endpoint;
	string region;
	string access_key_id;
	string secret_access_key;
	string session_token;
	string token;
};
} // namespace duckdb
