#include "readers/arxiv.h"
#include "source/options.hpp"
#include "source/readers/arxiv.hpp"
#include "storage_options.hpp"
namespace duckdb {
char *OpenArxiv(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                const TableFunctionBindInput &input, DuckportSource **output) {
	ArxivOptions reader_options {};
	auto kind = ReadStringOption(input, "kind");
	reader_options.kind = kind ? kind->c_str() : nullptr;
	auto month = ReadStringOption(input, "month");
	reader_options.month = month ? month->c_str() : nullptr;
	ReadBooleanOption(input, "requester_pays", reader_options.requester_pays);
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_arxiv_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
