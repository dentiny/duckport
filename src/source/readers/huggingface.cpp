#include "readers/huggingface.h"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
char *Open(ClientContext &context, const string &locator, const ::ScanOptions *scan,
		   const TableFunctionBindInput &input, DuckportSource **output) {
	string revision;
	string config;
	string split;
	::HuggingFaceDatasetOptions reader_options{};
	ReadStringOption(input, "revision", revision, reader_options.revision);
	ReadStringOption(input, "config", config, reader_options.config);
	ReadStringOption(input, "split", split, reader_options.split);
	ReadBooleanOption(input, "allow_partial", reader_options.allow_partial);
	StorageParameters storage(context, locator, "huggingface");
	return duckport_huggingface_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace
void RegisterReader_huggingface(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_huggingface", Open, [](TypedKwargs &options) {
		options.Add("revision", LogicalType::VARCHAR);
		options.Add("config", LogicalType::VARCHAR);
		options.Add("split", LogicalType::VARCHAR);
		options.Add("allow_partial", LogicalType::BOOLEAN);
	});
}
} // namespace duckdb
