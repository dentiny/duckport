#include "duckdb/common/string_util.hpp"
#include "readers/govinfo.h"
#include "source/readers/attachment.hpp"
#include "source/readers/govinfo.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenGovinfo(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                  const TableFunctionBindInput &input, DuckportSource **output) {
	GovinfoOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	StorageParameters storage(context, StringUtil::Format("%s/", common.Endpoint().value_or("https://www.govinfo.gov")),
	                          /*secret_type=*/ {});
	return duckport_govinfo_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
