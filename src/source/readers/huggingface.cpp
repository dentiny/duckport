#include "readers/huggingface.h"
#include "source/options.hpp"
#include "source/readers/huggingface.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenHuggingFace(ClientContext &context, const string &locator, const DuckportScanOptions *scan,
					  const TableFunctionBindInput &input, DuckportSource **output) {
	HuggingFaceDatasetOptions reader_options{};
	auto revision = ReadStringOption(input, "revision");
	reader_options.revision = revision ? revision->c_str() : nullptr;
	auto config = ReadStringOption(input, "config");
	reader_options.config = config ? config->c_str() : nullptr;
	auto split = ReadStringOption(input, "split");
	reader_options.split = split ? split->c_str() : nullptr;
	ReadBooleanOption(input, "allow_partial", reader_options.allow_partial);
	StorageParameters storage(context, locator, "huggingface");
	return duckport_huggingface_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace duckdb
