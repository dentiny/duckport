#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_xml(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_xml", duckport_xml_open,
				   [](TypedKwargs &options) { options.Add("record_path", LogicalType::VARCHAR); });
}
} // namespace duckdb
