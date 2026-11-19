#include "readers/dataverse.h"
#include "source/readers/attachment.hpp"
#include "source/readers/dataverse.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenDataverse(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                    const TableFunctionBindInput &input, DuckportSource **output) {
	DataverseOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto version = ReadStringOption(input, "version");
	reader_options.version = version ? version->c_str() : nullptr;
	StorageParameters storage(context, common.Endpoint().value_or("https://dataverse.harvard.edu") + "/",
	                          /*secret_type=*/ {});
	return duckport_dataverse_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
