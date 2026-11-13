#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_json(ExtensionLoader &loader) {
	RegisterReader(loader, "json", [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); },
	               {});
}
} // namespace duckdb
