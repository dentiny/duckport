#include "readers/openstax.h"
#include "source/readers/attachment.hpp"
#include "source/readers/openstax.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenOpenstax(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                   const TableFunctionBindInput &input, DuckportSource **output) {
	OpenstaxOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto revision = ReadStringOption(input, "revision");
	reader_options.revision = revision ? revision->c_str() : nullptr;
	auto archive_endpoint = ReadStringOption(input, "archive_endpoint");
	reader_options.archive_endpoint = archive_endpoint ? archive_endpoint->c_str() : nullptr;
	StorageParameters storage(context, common.Endpoint().value_or("https://api.github.com") + "/", /*secret_type=*/ {});
	return duckport_openstax_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
