#include "sink/copy.hpp"
#include "error.hpp"
#include <memory>

namespace duckdb {
namespace {
class LanceWriter final : public SinkWriter {
public:
	explicit LanceWriter(DuckportSink *writer) : writer(writer, duckport_sink_destroy) {
	}
	void Write(ArrowArray &array) override {
		CheckError(duckport_sink_write(writer.get(), &array));
	}
	void Commit() override {
		CheckError(duckport_sink_commit(writer.get()));
	}
	void Abort() noexcept override {
		writer.reset();
	}

private:
	std::unique_ptr<DuckportSink, decltype(&duckport_sink_destroy)> writer;
};
} // namespace
unique_ptr<SinkWriter> OpenLanceWriter(const string &path, const DuckportStorageOptions &storage,
                                       const SinkWriteOptions &options, ArrowSchema &schema) {
	const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
	                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
	                                                              : "create";
	vector<const char *> columns;
	for (const auto &column : options.blob_columns) {
		columns.push_back(column.c_str());
	}
	DuckportSink *raw = nullptr;
	CheckError(duckport_lance_open(path.c_str(), &schema, mode, columns.data(), columns.size(), &storage, &raw));
	return make_uniq<LanceWriter>(raw);
}
} // namespace duckdb
