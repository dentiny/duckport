#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_tsv(ExtensionLoader &loader) {
	RegisterReader(loader, "tsv",
	               [](TypedKwargs &options) {
		               options.Add("storage_options", LogicalType::VARCHAR);
		               options.Add("header", LogicalType::BOOLEAN);
	               },
	               {});
}
} // namespace duckdb
