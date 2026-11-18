#include "blob_secrets.hpp"
#include "duckdb/catalog/catalog_transaction.hpp"
#include "duckdb/common/string_util.hpp"
#include "duckdb/main/secret/secret_manager.hpp"
#include "storage_options.hpp"
namespace duckdb {
vector<BlobSecret> CaptureBlobSecrets(ClientContext &context) {
	auto entries = SecretManager::Get(context).AllSecrets(CatalogTransaction::GetSystemCatalogTransaction(context));
	vector<BlobSecret> result;
	auto policy = ReadStoragePolicy(context);
	for (const auto &entry : entries) {
		if (entry.secret->GetType() != "http" && entry.secret->GetType() != "s3") {
			continue;
		}
		auto secret = dynamic_cast<const KeyValueSecret *>(entry.secret.get());
		if (!secret) {
			continue;
		}
		BlobSecret value;
		value.type = secret->GetType().GetIdentifierName();
		value.policy = policy;
		if (value.type == "http") {
			value.security = ReadHttpSecurity(*secret);
		}
		if (value.type == "s3") {
			auto read = [&](const char *key, string &target) {
				Value field;
				if (secret->TryGetValue(key, field) && !field.IsNull()) {
					target = field.GetValue<string>();
				}
			};
			read("endpoint", value.endpoint);
			read("region", value.region);
			read("key_id", value.security.access_key_id);
			read("secret", value.security.secret_access_key);
			read("session_token", value.security.session_token);
			Value ssl;
			bool use_ssl = true;
			if (secret->TryGetValue("use_ssl", ssl) && !ssl.IsNull()) {
				use_ssl = ssl.GetValue<bool>();
			}
			if (!value.endpoint.empty() && value.endpoint.find("://") == string::npos) {
				value.endpoint = StringUtil::Format("%s://%s", use_ssl ? "https" : "http", value.endpoint);
			}
			string style;
			read("url_style", style);
			value.virtual_host_style = style == "vhost";
			value.security.allow_anonymous = value.security.access_key_id.empty() &&
											 value.security.secret_access_key.empty() &&
											 value.security.session_token.empty();
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
