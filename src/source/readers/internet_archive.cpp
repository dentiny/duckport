#include "duckdb/common/string_util.hpp"
#include "readers/internet_archive.h"
#include "source/options.hpp"
#include "source/readers/internet_archive.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenInternetArchive(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                          const TableFunctionBindInput &input, DuckportSource **output) {
	InternetArchiveOptions reader_options {};
	auto file_name = ReadStringOption(input, "file_name");
	reader_options.file_name = file_name ? file_name->c_str() : nullptr;
	auto format = ReadStringOption(input, "format");
	reader_options.format = format ? format->c_str() : nullptr;
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
	auto endpoint = ReadStringOption(input, "endpoint");
	reader_options.endpoint = endpoint ? endpoint->c_str() : nullptr;
	StorageParameters storage(context, StringUtil::Format("%s/", endpoint.value_or("https://archive.org")),
	                          /*secret_type=*/"internet_archive");
	return duckport_internet_archive_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
