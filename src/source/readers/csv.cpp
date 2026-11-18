#include "readers/csv.h"
#include "source/options.hpp"
#include "source/readers/csv.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
char *OpenDelimited(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                    const TableFunctionBindInput &input, DuckportSource **output, uint8_t delimiter) {
	CsvOptions reader_options {};
	reader_options.header = -1;
	reader_options.delimiter = delimiter;
	auto delimiter_option = ReadStringOption(input, "delimiter");
	if (delimiter_option) {
		if (delimiter_option->size() != 1) {
			throw BinderException("delimiter must be one ASCII character");
		}
		reader_options.delimiter = static_cast<uint8_t>((*delimiter_option)[0]);
	}
	ReadBooleanOption(input, "header", reader_options.header);
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_csv_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace
char *OpenCsv(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
              const TableFunctionBindInput &input, DuckportSource **output) {
	return OpenDelimited(context, locator, scan, input, output, ',');
}
char *OpenTsv(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
              const TableFunctionBindInput &input, DuckportSource **output) {
	return OpenDelimited(context, locator, scan, input, output, '\t');
}
} // namespace duckdb
