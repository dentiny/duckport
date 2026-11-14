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

ScanParameters::ScanParameters(const TableFunctionBindInput &input) {
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
	ReadStringOption(input, "revision", revision, scan.revision);
	ReadStringOption(input, "record_path", record_path, scan.record_path);
	ReadStringOption(input, "format", format, scan.format);
	ReadStringOption(input, "path", path, scan.path);
	ReadStringOption(input, "config", config, scan.config);
	ReadStringOption(input, "split", split, scan.split);
	ReadBooleanOption(input, "header", scan.header);
	ReadBooleanOption(input, "allow_partial", scan.allow_partial);
}

} // namespace duckdb
