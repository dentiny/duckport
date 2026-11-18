#include "error.hpp"
#include "source/source_factory.hpp"
#include "duckdb/main/query_profiler.hpp"

namespace duckdb {

SourceFactory::SourceFactory(SourcePtr source_p) : source(std::move(source_p)) {
}

optional<uint64_t> SourceFactory::ConfigureCopy(idx_t read_concurrency, bool inspect_size) {
	uint64_t bytes = 0;
	bool known = false;
	CheckError(duckport_source_configure_copy(source.get(), read_concurrency, inspect_size, &bytes, &known));
	return known ? optional<uint64_t>(bytes) : nullopt;
}

void SourceFactory::GetSchema(ArrowSchema &schema) {
	CheckError(duckport_source_get_schema(source.get(), &schema));
}

unique_ptr<ArrowArrayStreamWrapper> SourceFactory::ProduceStream(ArrowStreamParameters &parameters) {
	auto stream = make_uniq<ArrowArrayStreamWrapper>();
	vector<const char *> columns;
	columns.reserve(parameters.projected_columns.columns.size());
	for (const auto &column : parameters.projected_columns.columns) {
		columns.push_back(column.c_str());
	}
	CheckError(duckport_source_get_stream(source.get(), columns.data(), columns.size(), &stream->arrow_array_stream));
	return stream;
}

namespace {
uint64_t TakeDelta(atomic<uint64_t> &reported, uint64_t current) {
	auto previous = reported.load(std::memory_order_relaxed);
	while (current > previous && !reported.compare_exchange_weak(previous, current, std::memory_order_relaxed)) {
	}
	return current > previous ? current - previous : 0;
}
} // namespace
DuckportReadMetrics SourceFactory::Snapshot() const {
	DuckportReadMetrics metrics {};
	CheckError(duckport_source_get_metrics(source.get(), &metrics));
	return metrics;
}
void SourceFactory::ReportBytes(ClientContext &context) {
	auto delta = TakeDelta(reported_bytes, Snapshot().bytes_read);
	if (delta) {
		QueryProfiler::Get(context).TrackBytesRead(delta);
	}
}
uint64_t SourceFactory::ReportRows(uint64_t rows) {
	return TakeDelta(reported_rows, rows);
}
} // namespace duckdb
