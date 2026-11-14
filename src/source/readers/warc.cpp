#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_warc(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_warc", duckport_warc_open,
				   [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); });
}
} // namespace duckdb
