#include "sink/copy.hpp"
#include "storage_options.hpp"
#include "sink/writer_context.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenIcebergWriter(const string &path, WriterOpenContext &context,
                                         const SinkWriteOptions &options, ArrowSchema &schema) {
	const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
	                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
	                                                              : "create";
	vector<DuckportColumnEncoding> encodings;
	vector<DuckportIndexOptions> indexes;
	auto write_options = options.ToFfi(encodings, indexes);
	StorageParameters catalog(context.Client(), options.catalog_uri, "http");
	DuckportSink *raw = nullptr;
	CheckError(duckport_iceberg_open(path.c_str(), &schema, mode, options.blob_columns.size(),
	                                 options.catalog_uri.c_str(), &catalog.config.security, options.warehouse.c_str(),
	                                 options.storage_root.c_str(), &write_options, &context.storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
