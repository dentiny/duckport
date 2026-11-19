#include "sink/copy.hpp"
#include "storage_options.hpp"
#include "sink/writer_context.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenIcebergWriter(const string &path, WriterOpenContext &context,
                                         const SinkWriteOptions &options, ArrowSchema &schema) {
	SinkWriteOptionsFfiView write_options(options);
	StorageParameters catalog(context.Client(), options.catalog_uri, "http");
	DuckportSink *raw = nullptr;
	CheckError(duckport_iceberg_open(path.c_str(), &schema, write_options.Mode(), options.blob_columns.size(),
	                                 options.catalog_uri.c_str(), &catalog.config.security, options.warehouse.c_str(),
	                                 options.storage_root.c_str(), &write_options.Get(), &context.storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
