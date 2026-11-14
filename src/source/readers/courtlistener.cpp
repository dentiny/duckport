#include "readers/courtlistener.h"
#include "source/options.hpp"
#include "source/readers/courtlistener.hpp"
#include "storage_options.hpp"
namespace duckdb {
char *OpenCourtlistener(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                        const TableFunctionBindInput &input, DuckportSource **output) {
	CourtlistenerOptions reader_options {};
	auto table_name = ReadStringOption(input, "table_name");
	reader_options.table_name = table_name ? table_name->c_str() : nullptr;
	auto date = ReadStringOption(input, "date");
	reader_options.date = date ? date->c_str() : nullptr;
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_courtlistener_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
