#include "readers/bhl.h"
#include "source/readers/attachment.hpp"
#include "source/readers/bhl.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenBhl(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
              const TableFunctionBindInput &input, DuckportSource **output) {
	BhlOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	StorageParameters storage(context, common.Endpoint().value_or("https://www.biodiversitylibrary.org") + "/",
	                          /*secret_type=*/ {});
	return duckport_bhl_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
