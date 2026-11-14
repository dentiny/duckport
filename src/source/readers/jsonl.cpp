#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_jsonl(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_jsonl", duckport_jsonl_open,
				   [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); });
}
} // namespace duckdb
