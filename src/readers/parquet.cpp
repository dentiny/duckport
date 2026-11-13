#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_parquet(ExtensionLoader &loader) {
	RegisterReader(loader, "parquet",
	               [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); }, {});
}
} // namespace duckdb
