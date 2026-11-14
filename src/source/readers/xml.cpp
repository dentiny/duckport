#include "readers/xml.h"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "storage_options.hpp"

namespace duckdb {
namespace {
char *Open(ClientContext &context, const string &locator, const ::ScanOptions *scan,
		   const TableFunctionBindInput &input, DuckportSource **output) {
	::XmlOptions reader_options{};
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
	StorageParameters storage(context, locator, {});
	return duckport_xml_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace
void RegisterReader_xml(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_xml", Open,
				   [](TypedKwargs &options) { options.Add("record_path", LogicalType::VARCHAR); });
}
} // namespace duckdb
