#include "readers/ndjson.h"
#include "source/reader.hpp"

namespace duckdb {
void RegisterReader_ndjson(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_ndjson", duckport_ndjson_open);
}
} // namespace duckdb
