#include "readers/huggingface_file.h"
#include "source/options.hpp"
#include "source/readers/huggingface_file.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenHuggingFaceFile(ClientContext &context, const string &locator, const DuckportScanOptions *scan,
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
	StorageParameters storage(context, locator, "huggingface");
	return duckport_huggingface_file_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace duckdb
