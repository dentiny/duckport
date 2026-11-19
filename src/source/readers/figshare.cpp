#include "duckdb/common/string_util.hpp"
#include "readers/figshare.h"
#include "source/readers/attachment.hpp"
#include "source/readers/figshare.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenFigshare(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                   const TableFunctionBindInput &input, DuckportSource **output) {
	FigshareOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto version = ReadStringOption(input, "version");
	reader_options.version = version ? version->c_str() : nullptr;
	auto origin = common.Endpoint().value_or("https://api.figshare.com");
	while (!origin.empty() && origin.back() == '/') {
		origin.pop_back();
	}
	auto scope = StringUtil::Format("%s/v2/articles/%s", origin, locator);
	if (version && !version->empty()) {
		scope = StringUtil::Format("%s/versions/%s", scope, *version);
	}
	StorageParameters storage(context, scope, /*secret_type=*/ {});
	return duckport_figshare_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
