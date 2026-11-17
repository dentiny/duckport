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
	vector<DuckportColumnEncoding> encodings;
	vector<DuckportIndexOptions> indexes;
	auto write_options = options.ToFfi(encodings, indexes);
    vector<DuckportBlobSecret> secrets;
    vector<vector<DuckportHttpHeader>> headers(options.blob_secrets.size());
    for (idx_t i = 0; i < options.blob_secrets.size(); i++) {
        const auto &source = options.blob_secrets[i];
        for (idx_t h = 0; h < source.header_names.size(); h++) {
            headers[i].push_back({source.header_names[h].c_str(), source.header_values[h].c_str()});
        }
        DuckportStorageOptions config {};
        config.token = source.token.empty() ? nullptr : source.token.c_str();
        config.policy = source.policy;
        config.http_headers = headers[i].data();
        config.http_header_count = headers[i].size();
        secrets.push_back({source.scope.c_str(), source.type.c_str(), config});
    }
	DuckportSink *raw = nullptr;
	CheckError(duckport_lance_open(path.c_str(), &schema, mode, columns.data(), columns.size(), &write_options, secrets.data(), secrets.size(),
	                               &storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
