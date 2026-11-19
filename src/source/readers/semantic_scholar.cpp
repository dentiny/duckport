#include "duckdb/common/string_util.hpp"
#include "readers/semantic_scholar.h"
#include "source/readers/attachment.hpp"
#include "source/readers/semantic_scholar.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenSemanticScholar(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                          const TableFunctionBindInput &input, DuckportSource **output) {
	SemanticScholarOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	auto release = ReadStringOption(input, "release");
	reader_options.release = release ? release->c_str() : nullptr;
	StorageParameters storage(context,
	                          StringUtil::Format("%s/", common.Endpoint().value_or("https://api.semanticscholar.org")),
	                          /*secret_type=*/ {});
	return duckport_semantic_scholar_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
