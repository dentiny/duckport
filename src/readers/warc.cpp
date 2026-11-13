#include "duckport_reader.hpp"

namespace duckdb {
void RegisterReader_warc(ExtensionLoader &loader) {
	RegisterReader(loader, "warc", [](TypedKwargs &options) { options.Add("storage_options", LogicalType::VARCHAR); },
	               {});
}
} // namespace duckdb
