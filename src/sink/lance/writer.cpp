#include "error.hpp"
#include "sink/copy.hpp"
#include "source_metadata.hpp"
#include "storage_options.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenLanceWriter(ClientContext &context, const string &path,
                                       const DuckportStorageOptions &storage, const SinkWriteOptions &options,
                                       ArrowSchema &schema) {
	const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
	                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
	                                                              : "create";
	vector<const char *> columns;
	columns.reserve(options.blob_columns.size());
	for (const auto &column : options.blob_columns) {
		columns.push_back(column.c_str());
	}
	// Only Lance resolves media credentials and configures URL ingestion.
	const auto documents = SourceMediaDocuments(schema);
	vector<unique_ptr<StorageParameters>> source_storage;
	vector<DuckportBlobSource> sources;
	for (const auto &document : documents) {
		source_storage.push_back(make_uniq<StorageParameters>(context, document, string()));
		sources.push_back(DuckportBlobSource {document.c_str(), source_storage.back()->config});
	}
	vector<DuckportColumnEncoding> encodings;
	vector<DuckportIndexOptions> indexes;
	auto write_options = options.ToFfi(encodings, indexes);
	DuckportSink *raw = nullptr;
	CheckError(duckport_lance_open(path.c_str(), &schema, mode, columns.data(), columns.size(), &write_options,
	                               &storage, sources.data(), sources.size(), &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
