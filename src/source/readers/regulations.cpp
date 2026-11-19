#include "readers/regulations.h"
#include "source/readers/attachment.hpp"
#include "source/readers/regulations.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenRegulations(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                      const TableFunctionBindInput &input, DuckportSource **output) {
	RegulationsOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto resource = ReadStringOption(input, "resource");
	reader_options.resource = resource ? resource->c_str() : nullptr;
	StorageParameters storage(context, common.Endpoint().value_or("https://api.regulations.gov") + "/",
	                          /*secret_type=*/ {});
	return duckport_regulations_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
