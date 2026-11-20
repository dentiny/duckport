#include "duckdb/common/string_util.hpp"
#include "readers/semantic_scholar.h"
#include "source/options.hpp"
#include "source/readers/semantic_scholar.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenSemanticScholar(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                          const TableFunctionBindInput &input, DuckportSource **output) {
	auto file_name = ReadStringOption(input, "file_name");
	auto endpoint = ReadStringOption(input, "endpoint");
	auto release = ReadStringOption(input, "release");
	SemanticScholarOptions reader_options {file_name ? file_name->c_str() : nullptr,
	                                       endpoint ? endpoint->c_str() : nullptr,
	                                       release ? release->c_str() : nullptr};
	StorageParameters storage(context, StringUtil::Format("%s/", endpoint.value_or("https://api.semanticscholar.org")),
	                          /*secret_type=*/ {});
	return duckport_semantic_scholar_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
