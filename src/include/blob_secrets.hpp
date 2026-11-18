#pragma once
#include "duckdb/common/helper.hpp"
#include "duckdb/common/string.hpp"
#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/common/vector.hpp"
#include "duckport.h"
#include "security_options.hpp"
namespace duckdb {
class ClientContext;
struct BlobSecret {
	string scope;
	string type;
	string endpoint;
	string region;
	bool virtual_host_style = false;
	SecurityParameters security;
	DuckportStoragePolicy policy;
};
vector<BlobSecret> CaptureBlobSecrets(ClientContext &context, const string &secret_type = string());

// Owns a complete source snapshot and all nested C ABI views. File readers and
// Blob writers use the same owner; callers never manage scratch header arrays.
class SourceSecretsFfiView {
  public:
	explicit SourceSecretsFfiView(vector<BlobSecret> sources) : sources(std::move(sources)) {
		for (auto &source : this->sources) {
			security.push_back(make_uniq<SecurityFfiView>(std::move(source.security)));
			const auto &view = security.back()->Get();
			DuckportStorageOptions config{};
			config.endpoint = source.endpoint.empty() ? nullptr : source.endpoint.c_str();
			config.region = source.region.empty() ? nullptr : source.region.c_str();
			config.virtual_host_style = source.virtual_host_style;
			config.policy = source.policy;
			config.security = view;
			blobs.push_back({source.scope.c_str(), source.type.c_str(), config});
		}
	}
	SourceSecretsFfiView(const SourceSecretsFfiView &) = delete;
	SourceSecretsFfiView &operator=(const SourceSecretsFfiView &) = delete;
	SourceSecretsFfiView(SourceSecretsFfiView &&) = delete;
	SourceSecretsFfiView &operator=(SourceSecretsFfiView &&) = delete;

	// The caller captures an HTTP-only snapshot; this view only marshals the ABI.
	void BindHttp(DuckportStorageOptions &config) & {
		http.clear();
		for (const auto &source : blobs) {
			http.push_back({source.scope, source.config.security});
		}
		config.http_secrets = http.data();
		config.http_secret_count = http.size();
	}
	void BindHttp(DuckportStorageOptions &config) && = delete;
	const vector<DuckportBlobSecret> &Blobs() const & { return blobs; }
	const vector<DuckportBlobSecret> &Blobs() const && = delete;

  private:
	vector<BlobSecret> sources;
	vector<unique_ptr<SecurityFfiView>> security;
	vector<DuckportHttpSecret> http;
	vector<DuckportBlobSecret> blobs;
};
} // namespace duckdb
