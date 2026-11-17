#include "sink/copy.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenParquetWriter(const string &path, const DuckportStorageOptions &storage,
                                         const SinkWriteOptions &options, ArrowSchema &schema) {
	const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
	                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
	                                                              : "create";
	vector<DuckportColumnEncoding> encodings;
	vector<DuckportIndexOptions> indexes;
	auto write_options = options.ToFfi(encodings, indexes);
	DuckportSink *raw = nullptr;
	CheckError(duckport_parquet_open(path.c_str(), &schema, mode, options.blob_columns.size(), &write_options,
	                                 &storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
