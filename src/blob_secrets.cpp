#include "blob_secrets.hpp"
#include "storage_options.hpp"
#include "duckdb/main/secret/secret_manager.hpp"
#include "duckdb/catalog/catalog_transaction.hpp"
namespace duckdb {
vector<BlobSecret> CaptureBlobSecrets(ClientContext &context) {
	auto entries = SecretManager::Get(context).AllSecrets(CatalogTransaction::GetSystemCatalogTransaction(context));
	vector<BlobSecret> result;
	auto policy = ReadStoragePolicy(context);
	for (const auto &entry : entries) {
		if (entry.secret->GetType() != "http") {
			continue;
		}
		auto secret = dynamic_cast<const KeyValueSecret *>(entry.secret.get());
		if (!secret) {
			continue;
		}
		BlobSecret value;
		value.type = "http";
		value.policy = policy;
		value.security = ReadHttpSecurity(*secret);
		const auto &scopes = secret->GetScope();
		if (scopes.empty()) {
			result.push_back(value);
		}
		for (const auto &scope : scopes) {
			value.scope = scope;
			result.push_back(value);
		}
	}
	return result;
}
} // namespace duckdb
