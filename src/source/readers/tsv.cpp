#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_tsv(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_tsv", duckport_tsv_open,
				   [](TypedKwargs &options) { options.Add("header", LogicalType::BOOLEAN); });
}
} // namespace duckdb
