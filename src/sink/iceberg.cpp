#include "error.hpp"
#include "sink/copy.hpp"

namespace duckdb {
namespace {
class IcebergWriter final : public SinkWriter {
public:
	IcebergWriter(const string &identifier, const DuckportStorageOptions &storage, const SinkWriteOptions &options,
	              ArrowSchema &schema)
	    : writer(nullptr, duckport_sink_destroy) {
		DuckportSink *raw = nullptr;
		CheckError(duckport_iceberg_open(identifier.c_str(), &schema, options.catalog_uri.c_str(),
		                                 options.warehouse.c_str(), options.storage_root.c_str(), &storage, &raw));
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
unique_ptr<SinkWriter> OpenIcebergWriter(const string &path, const DuckportStorageOptions &storage,
                                         const SinkWriteOptions &options, ArrowSchema &schema) {
	return make_uniq<IcebergWriter>(path, storage, options, schema);
}
} // namespace duckdb
