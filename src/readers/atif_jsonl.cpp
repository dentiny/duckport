#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_atif_jsonl(ExtensionLoader &loader) {
	RegisterReader(loader, "atif_jsonl",
	               [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); }, {});
}
} // namespace duckdb
