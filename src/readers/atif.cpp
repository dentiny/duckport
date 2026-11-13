#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_atif(ExtensionLoader &loader) {
	RegisterReader(loader, "atif", [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); },
	               {});
}
} // namespace duckdb
