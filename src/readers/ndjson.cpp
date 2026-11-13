#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_ndjson(ExtensionLoader &loader) {
	RegisterReader(loader, "ndjson", [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); },
	               {});
}
} // namespace duckdb
