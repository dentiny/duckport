#include "readers/csv.h"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
char *Open(ClientContext &context, const string &locator, const DuckportScanOptions *scan,
		   const TableFunctionBindInput &input, DuckportSource **output) {
	CsvOptions reader_options{};
	reader_options.header = -1;
	ReadBooleanOption(input, "header", reader_options.header);
	StorageParameters storage(context, locator, {});
	return duckport_csv_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace
void RegisterReader_csv(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_csv", Open,
				   [](TypedKwargs &options) { options.Add("header", LogicalType::BOOLEAN); });
}
} // namespace duckdb
