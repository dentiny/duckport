#include "sink/copy.hpp"
#include "sink/writer_context.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenLanceWriter(const string &path, WriterOpenContext &context, const SinkWriteOptions &options,
                                       ArrowSchema &schema) {
	const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
	                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
	                                                              : "create";
	vector<const char *> columns;
	columns.reserve(options.blob_columns.size());
	for (const auto &column : options.blob_columns) {
		columns.push_back(column.c_str());
	}
	vector<DuckportColumnEncoding> encodings;
	vector<DuckportIndexOptions> indexes;
	auto write_options = options.ToFfi(encodings, indexes);
	auto sources = options.blob_columns.empty() ? vector<BlobSecret>() : context.BlobSecrets();
	SourceSecretsFfiView source_view(std::move(sources));
	const auto &secrets = source_view.Blobs();
	DuckportSink *raw = nullptr;
	CheckError(duckport_lance_open(path.c_str(), &schema, mode, columns.data(), columns.size(), &write_options,
	                               secrets.data(), secrets.size(), &context.storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
