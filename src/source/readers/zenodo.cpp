#include "readers/zenodo.h"
#include "source/options.hpp"
#include "source/readers/zenodo.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenZenodo(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                 const TableFunctionBindInput &input, DuckportSource **output) {
	ZenodoOptions reader_options {};
	auto file_name = ReadStringOption(input, "file_name");
	reader_options.file_name = file_name ? file_name->c_str() : nullptr;
	auto format = ReadStringOption(input, "format");
	reader_options.format = format ? format->c_str() : nullptr;
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
	auto endpoint = ReadStringOption(input, "endpoint");
	reader_options.endpoint = endpoint ? endpoint->c_str() : nullptr;
	StorageParameters storage(context, endpoint.value_or("https://zenodo.org") + "/", /*secret_type=*/ {});
	return duckport_zenodo_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
