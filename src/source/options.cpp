#include "source/options.hpp"

namespace duckdb {
namespace {
void ReadStringOption(const TableFunctionBindInput &input, const char *name, string &owned, const char *&output) {
	auto entry = input.named_parameters.find(name);
	if (entry != input.named_parameters.end()) {
		owned = ReadSourceString(entry->second);
		output = owned.c_str();
	}
}
void ReadBooleanOption(const TableFunctionBindInput &input, const char *name, int32_t &output) {
	auto entry = input.named_parameters.find(name);
	if (entry != input.named_parameters.end()) {
		output = BooleanValue::Get(entry->second) ? 1 : 0;
	}
}
} // namespace

string ReadSourceString(const Value &value) {
	if (value.IsNull()) {
		throw BinderException("DuckPort arguments cannot be NULL");
	}
	auto text = StringValue::Get(value);
	if (text.find('\0') != string::npos) {
		throw BinderException("DuckPort arguments cannot contain NUL bytes");
	}
	return text;
}

ReaderOptions::ReaderOptions(const TableFunctionBindInput &input) {
	for (const auto &entry : input.named_parameters) {
		if (entry.second.IsNull()) {
			throw BinderException("DuckPort arguments cannot be NULL");
		}
	}
	constexpr idx_t DEFAULT_BATCH_SIZE = 8192;
	scan.batch_size = DEFAULT_BATCH_SIZE;
	scan.header = -1;
	auto batch = input.named_parameters.find("batch_size");
	if (batch != input.named_parameters.end()) {
		scan.batch_size = idx_t(BigIntValue::Get(batch->second));
	}
	ReadStringOption(input, "endpoint", endpoint, storage.endpoint);
	ReadStringOption(input, "region", region, storage.region);
	ReadStringOption(input, "access_key_id", access_key_id, storage.access_key_id);
	ReadStringOption(input, "secret_access_key", secret_access_key, storage.secret_access_key);
	ReadStringOption(input, "session_token", session_token, storage.session_token);
	ReadStringOption(input, "token", token, storage.token);
	ReadStringOption(input, "revision", revision, storage.revision);
	ReadBooleanOption(input, "virtual_host_style", storage.virtual_host_style);
	ReadBooleanOption(input, "allow_anonymous", storage.allow_anonymous);
	ReadStringOption(input, "record_path", record_path, scan.record_path);
	ReadStringOption(input, "format", format, scan.format);
	ReadStringOption(input, "path", path, scan.path);
	ReadStringOption(input, "config", config, scan.config);
	ReadStringOption(input, "split", split, scan.split);
	ReadBooleanOption(input, "header", scan.header);
	ReadBooleanOption(input, "allow_partial", scan.allow_partial);
}

void RegisterStorageOptions(TypedKwargs &options) {
	options.Add("endpoint", LogicalType::VARCHAR);
	options.Add("region", LogicalType::VARCHAR);
	options.Add("access_key_id", LogicalType::VARCHAR);
	options.Add("secret_access_key", LogicalType::VARCHAR);
	options.Add("session_token", LogicalType::VARCHAR);
	options.Add("token", LogicalType::VARCHAR);
	options.Add("virtual_host_style", LogicalType::BOOLEAN);
	options.Add("allow_anonymous", LogicalType::BOOLEAN);
}
} // namespace duckdb
