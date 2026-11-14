#include "readers/huggingface_file.h"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
char *Open(ClientContext &context, const string &locator, const DuckportScanOptions *scan,
		   const TableFunctionBindInput &input, DuckportSource **output) {
	HuggingFaceFileOptions reader_options{};
	reader_options.header = -1;
	auto revision = ReadStringOption(input, "revision");
	reader_options.revision = revision ? revision->c_str() : nullptr;
	auto path = ReadStringOption(input, "path");
	reader_options.path = path ? path->c_str() : nullptr;
	auto format = ReadStringOption(input, "format");
	reader_options.format = format ? format->c_str() : nullptr;
	ReadBooleanOption(input, "header", reader_options.header);
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
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
