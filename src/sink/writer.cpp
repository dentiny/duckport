#include "sink/writer.hpp"
#include "sink/writer_context.hpp"
#include "error.hpp"

namespace duckdb {
const vector<BlobSecret> &WriterOpenContext::BlobSecrets() {
	if (!blob_secrets) {
		blob_secrets = CaptureBlobSecrets(client);
	}
	return *blob_secrets;
}

namespace {
class RustSinkWriter final : public SinkWriter {
public:
	explicit RustSinkWriter(SinkPtr writer) : writer(std::move(writer)) {
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
	SinkPtr writer;
};
} // namespace
unique_ptr<SinkWriter> MakeSinkWriter(SinkPtr writer) {
	return make_uniq<RustSinkWriter>(std::move(writer));
}
} // namespace duckdb
