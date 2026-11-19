#include "duckdb/common/string_util.hpp"
#include "readers/library_of_congress.h"
#include "source/readers/attachment.hpp"
#include "source/readers/library_of_congress.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenLibraryOfCongress(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                            const TableFunctionBindInput &input, DuckportSource **output) {
	LibraryOfCongressOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	StorageParameters storage(context, StringUtil::Format("%s/", common.Endpoint().value_or("https://www.loc.gov")),
	                          /*secret_type=*/ {});
	return duckport_library_of_congress_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
