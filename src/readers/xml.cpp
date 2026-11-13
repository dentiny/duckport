#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_xml(ExtensionLoader &loader) {
	RegisterReader(loader, "xml",
	               [](TypedKwargs &options) {
		               options.Add("storage_options", LogicalType::VARCHAR);
		               options.Add("record_path", LogicalType::VARCHAR);
	               },
	               {"record_path"});
}
} // namespace duckdb
