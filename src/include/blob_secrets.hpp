#pragma once
#include "duckdb/common/string.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"
#include "security_options.hpp"
#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/common/helper.hpp"
namespace duckdb {
class ClientContext;
struct BlobSecret {
	string scope;
	string type;
	SecurityParameters security;
	DuckportStoragePolicy policy;
};
vector<BlobSecret> CaptureBlobSecrets(ClientContext &context);

// Owns a complete source snapshot and all nested C ABI views. File readers and
// Blob writers use the same owner; callers never manage scratch header arrays.
class SourceSecretsFfiView {
public:
	explicit SourceSecretsFfiView(vector<BlobSecret> sources) : sources(std::move(sources)) {
		for (auto &source : this->sources) {
			security.push_back(make_uniq<SecurityFfiView>(std::move(source.security)));
			const auto &view = security.back()->Get();
			http.push_back({source.scope.c_str(), view});
			DuckportStorageOptions config {};
			config.policy = source.policy;
			config.security = view;
			blobs.push_back({source.scope.c_str(), source.type.c_str(), config});
		}
	}
	SourceSecretsFfiView(const SourceSecretsFfiView &) = delete;
	SourceSecretsFfiView &operator=(const SourceSecretsFfiView &) = delete;
	SourceSecretsFfiView(SourceSecretsFfiView &&) = delete;
	SourceSecretsFfiView &operator=(SourceSecretsFfiView &&) = delete;

	void BindHttp(DuckportStorageOptions &config) const & {
		config.http_secrets = http.data();
		config.http_secret_count = http.size();
	}
	void BindHttp(DuckportStorageOptions &config) const && = delete;
	const vector<DuckportBlobSecret> &Blobs() const & {
		return blobs;
	}
	const vector<DuckportBlobSecret> &Blobs() const && = delete;

private:
	vector<BlobSecret> sources;
	vector<unique_ptr<SecurityFfiView>> security;
	vector<DuckportHttpSecret> http;
	vector<DuckportBlobSecret> blobs;
};
} // namespace duckdb
