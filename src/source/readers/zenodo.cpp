#include "duckdb/common/string_util.hpp"
#include "readers/zenodo.h"
#include "source/readers/attachment.hpp"
#include "source/readers/zenodo.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenZenodo(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                 const TableFunctionBindInput &input, DuckportSource **output) {
	ZenodoOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto origin = common.Endpoint().value_or("https://zenodo.org");
	if (!origin.empty() && origin.back() == '/') {
		origin.pop_back();
	}
	StorageParameters storage(context, StringUtil::Format("%s/api/records/%s", origin, locator), /*secret_type=*/ {});
	return duckport_zenodo_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
