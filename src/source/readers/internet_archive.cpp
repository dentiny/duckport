#include "duckdb/common/string_util.hpp"
#include "readers/internet_archive.h"
#include "source/readers/attachment.hpp"
#include "source/readers/internet_archive.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenInternetArchive(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                          const TableFunctionBindInput &input, DuckportSource **output) {
	InternetArchiveOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	StorageParameters storage(context, StringUtil::Format("%s/", common.Endpoint().value_or("https://archive.org")),
	                          /*secret_type=*/"internet_archive");
	return duckport_internet_archive_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
