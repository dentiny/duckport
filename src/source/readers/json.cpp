#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_json(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_json", duckport_json_open,
				   [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); });
}
} // namespace duckdb
