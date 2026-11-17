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
		Value token;
		if (secret->TryGetValue("bearer_token", token) && !token.IsNull()) {
			value.security.token = token.GetValue<string>();
            value.security.has_token = true;
		}
		Value headers;
		if (secret->TryGetValue("extra_http_headers", headers) && !headers.IsNull()) {
			for (const auto &item : MapValue::GetChildren(headers)) {
				const auto &pair = StructValue::GetChildren(item);
				if (pair[0].IsNull() || pair[1].IsNull()) {
					throw InvalidInputException("HTTP header names and values must not be NULL");
				}
				value.security.header_names.push_back(pair[0].GetValue<string>());
				value.security.header_values.push_back(pair[1].GetValue<string>());
			}
		}
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
