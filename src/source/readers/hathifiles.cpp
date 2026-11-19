#include "duckdb/common/string_util.hpp"
#include "readers/hathifiles.h"
#include "source/readers/attachment.hpp"
#include "source/readers/hathifiles.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenHathifiles(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                     const TableFunctionBindInput &input, DuckportSource **output) {
	HathifilesOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	StorageParameters storage(context,
	                          StringUtil::Format("%s/", common.Endpoint().value_or("https://www.hathitrust.org")),
	                          /*secret_type=*/ {});
	return duckport_hathifiles_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
