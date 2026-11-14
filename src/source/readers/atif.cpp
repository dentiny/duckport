#include "readers/atif.h"
#include "source/options.hpp"
#include "source/readers/atif.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenAtif(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
			   const TableFunctionBindInput &input, DuckportSource **output) {
	AtifOptions reader_options{};
	ReadBooleanOption(input, "jsonl", reader_options.jsonl);
	StorageParameters storage(context, locator, /*secret_type=*/{});
	return duckport_atif_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
