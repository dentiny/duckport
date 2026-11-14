#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_atif(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_atif", duckport_atif_open,
				   [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); });
}
} // namespace duckdb
