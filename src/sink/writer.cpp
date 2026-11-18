#include "sink/writer.hpp"
#include "error.hpp"

namespace duckdb {
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
	DuckportWriteMetrics Metrics() const override {
		DuckportWriteMetrics metrics {};
		CheckError(duckport_sink_get_metrics(writer.get(), &metrics));
		return metrics;
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
