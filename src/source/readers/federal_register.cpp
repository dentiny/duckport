#include "readers/federal_register.h"
#include "source/options.hpp"
#include "source/readers/federal_register.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenFederalRegister(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                          const TableFunctionBindInput &input, DuckportSource **output) {
	FederalRegisterOptions reader_options {};
	auto file_name = ReadStringOption(input, "file_name");
	reader_options.file_name = file_name ? file_name->c_str() : nullptr;
	auto format = ReadStringOption(input, "format");
	reader_options.format = format ? format->c_str() : nullptr;
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
	auto endpoint = ReadStringOption(input, "endpoint");
	reader_options.endpoint = endpoint ? endpoint->c_str() : nullptr;
	StorageParameters storage(context, endpoint.value_or("https://www.federalregister.gov") + "/", /*secret_type=*/ {});
	return duckport_federal_register_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
