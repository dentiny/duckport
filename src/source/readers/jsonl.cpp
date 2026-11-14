#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_jsonl(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_jsonl", duckport_jsonl_open);
}
} // namespace duckdb
