#define DUCKDB_EXTENSION_MAIN
#include "duckport_extension.hpp"
#include "source/reader.hpp"
#include "source/readers/csv.hpp"

namespace duckdb {
namespace {
void LoadInternal(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_parquet", duckport_parquet_open);
	auto add_csv_options = [](TypedKwargs &options) { options.Add("header", LogicalType::BOOLEAN); };
	RegisterReader(loader, "duckport_scan_csv", OpenCsv, add_csv_options);
	RegisterReader(loader, "duckport_scan_tsv", OpenTsv, add_csv_options);
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
