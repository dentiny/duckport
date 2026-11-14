#include "readers/uspto.h"
#include "source/options.hpp"
#include "source/readers/uspto.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenUspto(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                const TableFunctionBindInput &input, DuckportSource **output) {
	UsptoOptions reader_options {};
	auto file_name = ReadStringOption(input, "file_name");
	reader_options.file_name = file_name ? file_name->c_str() : nullptr;
	auto format = ReadStringOption(input, "format");
	reader_options.format = format ? format->c_str() : nullptr;
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
	auto endpoint = ReadStringOption(input, "endpoint");
	reader_options.endpoint = endpoint ? endpoint->c_str() : nullptr;
	auto offset = ReadStringOption(input, "offset");
	reader_options.offset = offset ? offset->c_str() : nullptr;
	auto page_size = ReadStringOption(input, "page_size");
	reader_options.page_size = page_size ? page_size->c_str() : nullptr;
	StorageParameters storage(context, endpoint.value_or("https://api.uspto.gov") + "/", /*secret_type=*/ {});
	return duckport_uspto_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
