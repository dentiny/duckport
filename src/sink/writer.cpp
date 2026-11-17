#include "sink/copy.hpp"
#include "error.hpp"
#include "duckdb/common/unique_ptr.hpp"

namespace duckdb {
namespace {
class RustSinkWriter final : public SinkWriter {
public:
	RustSinkWriter(const char *format, const string &path, const DuckportStorageOptions &storage,
	               const SinkWriteOptions &options, ArrowSchema &schema)
	    : writer(nullptr, duckport_sink_destroy) {
		const char *mode = options.mode == SinkWriteMode::APPEND      ? "append"
		                   : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
		                                                              : "create";
		vector<const char *> columns;
		columns.reserve(options.blob_columns.size());
		for (const auto &column : options.blob_columns) {
			columns.push_back(column.c_str());
		}
		DuckportSink *raw = nullptr;
		CheckError(
		    duckport_sink_open(format, path.c_str(), &schema, mode, columns.data(), columns.size(), &storage, &raw));
		writer.reset(raw);
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
	unique_ptr<DuckportSink, decltype(&duckport_sink_destroy)> writer;
};
} // namespace
unique_ptr<SinkWriter> OpenSinkWriter(const char *format, const string &path, const DuckportStorageOptions &storage,
                                      const SinkWriteOptions &options, ArrowSchema &schema) {
	return make_uniq<RustSinkWriter>(format, path, storage, options, schema);
}
} // namespace duckdb
