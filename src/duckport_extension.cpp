#define DUCKDB_EXTENSION_MAIN
#include "duckport_extension.hpp"
#include "source/reader.hpp"
#include "source/readers/huggingface_file.hpp"

namespace duckdb {
namespace {
void LoadInternal(ExtensionLoader &loader) {
	RegisterReader(loader, "duckport_scan_parquet", duckport_parquet_open);
	RegisterReader(loader, "duckport_scan_huggingface_file", OpenHuggingFaceFile, [](TypedKwargs &options) {
		options.Add("revision", LogicalType::VARCHAR);
		options.Add("path", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("header", LogicalType::BOOLEAN);
		options.Add("record_path", LogicalType::VARCHAR);
	});
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
