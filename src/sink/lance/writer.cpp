#include "sink/copy.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenLanceWriter(const string &path, const DuckportStorageOptions &storage,
                                       const SinkWriteOptions &options, ArrowSchema &schema) {
	const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
	                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
	                                                              : "create";
	vector<const char *> columns;
	columns.reserve(options.blob_columns.size());
	for (const auto &column : options.blob_columns) {
		columns.push_back(column.c_str());
	}
	DuckportSink *raw = nullptr;
	CheckError(duckport_lance_open(path.c_str(), &schema, mode, columns.data(), columns.size(), &storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
