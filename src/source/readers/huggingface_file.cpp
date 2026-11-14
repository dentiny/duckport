#include "readers/huggingface_file.h"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
char *Open(ClientContext &context, const string &locator, const ::ScanOptions *scan,
		   const TableFunctionBindInput &input, DuckportSource **output) {
	string revision;
	string path;
	string format;
	string record_path;
	::HuggingFaceFileOptions reader_options{};
	reader_options.header = -1;
	ReadStringOption(input, "revision", revision, reader_options.revision);
	ReadStringOption(input, "path", path, reader_options.path);
	ReadStringOption(input, "format", format, reader_options.format);
	ReadBooleanOption(input, "header", reader_options.header);
	ReadStringOption(input, "record_path", record_path, reader_options.record_path);
	StorageParameters storage(context, locator, "huggingface", reader_options.path);
	return duckport_huggingface_file_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace
void RegisterReader_huggingface_file(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_huggingface_file", Open, [](TypedKwargs &options) {
		options.Add("revision", LogicalType::VARCHAR);
		options.Add("path", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("header", LogicalType::BOOLEAN);
		options.Add("record_path", LogicalType::VARCHAR);
	});
}
} // namespace duckdb
