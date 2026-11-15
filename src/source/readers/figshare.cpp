#include "readers/figshare.h"
#include "source/options.hpp"
#include "source/readers/figshare.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenFigshare(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                   const TableFunctionBindInput &input, DuckportSource **output) {
	FigshareOptions reader_options {};
	auto file_name = ReadStringOption(input, "file_name");
	reader_options.file_name = file_name ? file_name->c_str() : nullptr;
	auto format = ReadStringOption(input, "format");
	reader_options.format = format ? format->c_str() : nullptr;
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
	auto endpoint = ReadStringOption(input, "endpoint");
	reader_options.endpoint = endpoint ? endpoint->c_str() : nullptr;
	auto version = ReadStringOption(input, "version");
	reader_options.version = version ? version->c_str() : nullptr;
	auto origin = endpoint.value_or("https://api.figshare.com");
	while (!origin.empty() && origin.back() == '/') {
		origin.pop_back();
	}
	auto scope = origin + "/v2/articles/" + locator;
	if (version && !version->empty()) {
		scope += "/versions/" + *version;
	}
	StorageParameters storage(context, scope, /*secret_type=*/ {});
	return duckport_figshare_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
