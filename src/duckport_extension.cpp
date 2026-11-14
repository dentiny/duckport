#define DUCKDB_EXTENSION_MAIN
#include "duckport_extension.hpp"
#include "readers/ndjson.h"
#include "source/reader.hpp"

namespace duckdb {
namespace {
void LoadInternal(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_parquet", duckport_parquet_open);
	RegisterReader(loader, "duckport_scan_ndjson", duckport_ndjson_open);
}
} // namespace
void DuckportExtension::Load(ExtensionLoader &loader) { LoadInternal(loader); }
std::string DuckportExtension::Name() { return "duckport"; }
std::string DuckportExtension::Version() const {
#ifdef EXT_VERSION_DUCKPORT
	return EXT_VERSION_DUCKPORT;
#else
	return "";
#endif
}
} // namespace duckdb
extern "C" {
DUCKDB_CPP_EXTENSION_ENTRY(duckport, loader) { duckdb::LoadInternal(loader); }
}
