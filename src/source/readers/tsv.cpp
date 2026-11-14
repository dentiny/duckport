#include "readers/tsv.h"
#include "source/options.hpp"
#include "source/readers/tsv.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenTsv(ClientContext &context, const string &locator, const DuckportScanOptions *scan,
			  const TableFunctionBindInput &input, DuckportSource **output) {
	TsvOptions reader_options{};
	reader_options.header = -1;
	ReadBooleanOption(input, "header", reader_options.header);
	StorageParameters storage(context, locator, {});
	return duckport_tsv_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace duckdb
