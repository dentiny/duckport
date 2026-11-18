#pragma once

#include "blob_secrets.hpp"
#include "duckdb/common/optional.hpp"

namespace duckdb {
// Exists only for one writer open. Destination options are borrowed from the
// caller; source secrets are owned here and captured only when requested.
class WriterOpenContext {
public:
	WriterOpenContext(ClientContext &client, const DuckportStorageOptions &storage) : storage(storage), client(client) {
	}
	WriterOpenContext(const WriterOpenContext &) = delete;
	WriterOpenContext &operator=(const WriterOpenContext &) = delete;

	const DuckportStorageOptions &storage;
	const vector<BlobSecret> &BlobSecrets();
	ClientContext &Client() { return client; }

private:
	ClientContext &client;
	optional<vector<BlobSecret>> blob_secrets;
};
} // namespace duckdb
