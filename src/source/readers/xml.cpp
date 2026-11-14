#include "readers/xml.h"
#include "source/options.hpp"
#include "source/readers/xml.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenXml(ClientContext &context, const string &locator, const DuckportScanOptions *scan,
			  const TableFunctionBindInput &input, DuckportSource **output) {
	XmlOptions reader_options{};
	auto record_path = ReadStringOption(input, "record_path");
	reader_options.record_path = record_path ? record_path->c_str() : nullptr;
	StorageParameters storage(context, locator, {});
	return duckport_xml_open(locator.c_str(), &storage.config, scan, &reader_options, output);
}
} // namespace duckdb
