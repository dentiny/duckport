#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_http(ExtensionLoader &loader) {
	RegisterReader(loader, "http",
	               [](TypedKwargs &options) {
		               options.Add("storage_options", LogicalType::VARCHAR);
		               options.Add("header", LogicalType::BOOLEAN);
		               options.Add("record_path", LogicalType::VARCHAR);
		               options.Add("format", LogicalType::VARCHAR);
	               },
	               {"format"});
}
} // namespace duckdb
