#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_huggingface(ExtensionLoader &loader) {
	RegisterReader(
		loader, "duckport_scan_huggingface", duckport_huggingface_open,
		[](TypedKwargs &options) {
			options.Add("revision", LogicalType::VARCHAR);
			options.Add("config", LogicalType::VARCHAR);
			options.Add("split", LogicalType::VARCHAR);
			options.Add("allow_partial", LogicalType::BOOLEAN);
		},
		"huggingface");
}
} // namespace duckdb
