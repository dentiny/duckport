#include "error.hpp"
#include "sink/copy.hpp"
#include "sink/writer_context.hpp"
#include "source_metadata.hpp"
#include "storage_options.hpp"

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
	const auto documents = SourceMediaDocuments(schema);
	vector<unique_ptr<StorageParameters>> source_storage;
	vector<DuckportBlobSource> media_sources;
	for (const auto &document : documents) {
		source_storage.push_back(make_uniq<StorageParameters>(context.Client(), document, string()));
		media_sources.push_back({document.c_str(), source_storage.back()->config});
	}
	vector<DuckportColumnEncoding> encodings;
	vector<DuckportIndexOptions> indexes;
	auto write_options = options.ToFfi(encodings, indexes);
	auto sources = options.blob_columns.empty() && documents.empty() ? vector<BlobSecret>() : context.BlobSecrets();
	SourceSecretsFfiView source_view(std::move(sources));
	const auto &secrets = source_view.Blobs();
	DuckportSink *raw = nullptr;
	CheckError(duckport_lance_open(path.c_str(), &schema, mode, columns.data(), columns.size(), &write_options,
	                               secrets.data(), secrets.size(), &context.storage, media_sources.data(),
	                               media_sources.size(), &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
