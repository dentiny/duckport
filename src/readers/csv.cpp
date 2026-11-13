#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_csv(ExtensionLoader &loader) {
	RegisterReader(loader, "csv",
	               [](TypedKwargs &options) {
		               options.Add("storage_options", LogicalType::VARCHAR);
		               options.Add("header", LogicalType::BOOLEAN);
	               },
	               {});
}
} // namespace duckdb
