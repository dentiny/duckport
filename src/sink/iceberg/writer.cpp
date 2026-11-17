#include "sink/copy.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenIcebergWriter(const string &path, const DuckportStorageOptions &storage,
                                         const SinkWriteOptions &options, ArrowSchema &schema) {
	DuckportSink *raw = nullptr;
	CheckError(duckport_iceberg_open(path.c_str(), &schema, options.catalog_uri.c_str(), options.warehouse.c_str(),
	                                 options.storage_root.c_str(), &storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
