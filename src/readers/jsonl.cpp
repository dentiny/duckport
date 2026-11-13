#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_jsonl(ExtensionLoader &loader) {
	RegisterReader(loader, "jsonl", [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); },
	               {});
}
} // namespace duckdb
