#include "duckdb/common/string_util.hpp"
#include "readers/uspto.h"
#include "source/readers/attachment.hpp"
#include "source/readers/uspto.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenUspto(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                const TableFunctionBindInput &input, DuckportSource **output) {
	UsptoOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto offset = ReadStringOption(input, "offset");
	reader_options.offset = offset ? offset->c_str() : nullptr;
	auto page_size = ReadStringOption(input, "page_size");
	reader_options.page_size = page_size ? page_size->c_str() : nullptr;
	StorageParameters storage(context, StringUtil::Format("%s/", common.Endpoint().value_or("https://api.uspto.gov")),
	                          /*secret_type=*/ {});
	return duckport_uspto_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
