#include "sink/copy.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenParquetWriter(const string &path, const DuckportStorageOptions &storage,
                                         const SinkWriteOptions &options, ArrowSchema &schema) {
	if (!options.blob_columns.empty()) {
		throw BinderException("Parquet sink does not support BLOB_COLUMNS");
	}
	const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
	                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
	                                                              : "create";
	DuckportSink *raw = nullptr;
	CheckError(duckport_parquet_open(path.c_str(), &schema, mode, &options.write, &storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
