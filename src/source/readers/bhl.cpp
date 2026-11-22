#include "duckdb/common/string_util.hpp"
#include "readers/bhl.h"
#include "source/readers/attachment.hpp"
#include "source/readers/bhl.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenBhl(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
              const TableFunctionBindInput &input, DuckportSource **output) {
	BhlOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	StorageParameters storage(
	    context, StringUtil::Format("%s/", common.Endpoint().value_or("https://www.biodiversitylibrary.org")),
	    /*secret_type=*/ {});
	return duckport_bhl_open(locator.c_str(), storage.config, scan, reader_options, output);
}

char *OpenBhlOpenData(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                      const TableFunctionBindInput &input, DuckportSource **output) {
	vector<string> keys;
	auto entry = input.named_parameters.find("keys");
	if (entry != input.named_parameters.end()) {
		for (const auto &key : ListValue::GetChildren(entry->second)) {
			keys.push_back(ReadSourceString(key));
		}
	}
	vector<const char *> pointers;
	pointers.reserve(keys.size());
	for (const auto &key : keys) {
		pointers.push_back(key.c_str());
	}
	auto format = ReadStringOption(input, "format");
	auto record_path = ReadStringOption(input, "record_path");
	BhlOpenDataOptions options {pointers.data(), pointers.size(), format ? format->c_str() : nullptr,
	                            record_path ? record_path->c_str() : nullptr};
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_bhl_open_data_open(locator.c_str(), storage.config, scan, options, output);
}
char *OpenBhlPages(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                   const TableFunctionBindInput &input, DuckportSource **output) {
	auto prefix = ReadStringOption(input, "prefix");
	auto database = ReadStringOption(input, "metadata_db");
	BhlPagesOptions options {prefix ? prefix->c_str() : nullptr, database ? database->c_str() : nullptr};
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_bhl_pages_open(locator.c_str(), storage.config, scan, options, output);
}
} // namespace duckdb
