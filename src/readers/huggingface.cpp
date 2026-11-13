#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_huggingface(ExtensionLoader &loader) {
	RegisterReader(loader, "huggingface",
	               [](TypedKwargs &options) {
		               options.Add("revision", LogicalType::VARCHAR);
		               options.Add("endpoint", LogicalType::VARCHAR);
		               options.Add("token", LogicalType::VARCHAR);
		               options.Add("config", LogicalType::VARCHAR);
		               options.Add("split", LogicalType::VARCHAR);
		               options.Add("allow_partial", LogicalType::BOOLEAN);
	               },
	               {});
}
} // namespace duckdb
