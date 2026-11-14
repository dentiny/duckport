#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_atif_jsonl(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_atif_jsonl", duckport_atif_jsonl_open,
				   [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); });
}
} // namespace duckdb
