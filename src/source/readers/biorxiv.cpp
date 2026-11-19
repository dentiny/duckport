#include "duckdb/common/string_util.hpp"
#include "readers/biorxiv.h"
#include "source/readers/attachment.hpp"
#include "source/readers/biorxiv.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenBiorxiv(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                  const TableFunctionBindInput &input, DuckportSource **output) {
	BiorxivOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto until = ReadStringOption(input, "until");
	reader_options.until = until ? until->c_str() : nullptr;
	auto server = ReadStringOption(input, "server");
	reader_options.server = server ? server->c_str() : nullptr;
	auto cursor = ReadStringOption(input, "cursor");
	reader_options.cursor = cursor ? cursor->c_str() : nullptr;
	StorageParameters storage(context, StringUtil::Format("%s/", common.Endpoint().value_or("https://api.biorxiv.org")),
	                          /*secret_type=*/ {});
	return duckport_biorxiv_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
