#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_huggingface_file(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_huggingface_file", duckport_huggingface_file_open,
				   [](TypedKwargs &options) {
					   options.Add("header", LogicalType::BOOLEAN);
					   options.Add("record_path", LogicalType::VARCHAR);
					   options.Add("revision", LogicalType::VARCHAR);
					   options.Add("endpoint", LogicalType::VARCHAR);
					   options.Add("token", LogicalType::VARCHAR);
					   options.Add("path", LogicalType::VARCHAR);
					   options.Add("format", LogicalType::VARCHAR);
				   },
				   {"endpoint", "token", "revision"});
}
} // namespace duckdb
