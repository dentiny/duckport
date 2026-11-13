#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_csv(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_csv", duckport_csv_open, [](TypedKwargs &options) {
		options.Add("storage_options", LogicalType::VARCHAR);
		options.Add("header", LogicalType::BOOLEAN);
	});
}
} // namespace duckdb
