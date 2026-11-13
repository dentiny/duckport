#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_s3(ExtensionLoader &loader) {
	RegisterReader(loader, "s3",
	               [](TypedKwargs &options) {
		               options.Add("storage_options", LogicalType::VARCHAR);
		               options.Add("header", LogicalType::BOOLEAN);
		               options.Add("record_path", LogicalType::VARCHAR);
		               options.Add("format", LogicalType::VARCHAR);
	               },
	               {"format"});
}
} // namespace duckdb
