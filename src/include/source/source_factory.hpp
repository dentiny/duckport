#pragma once

#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/function/table/arrow.hpp"
#include "duckport.h"
#include <atomic>

namespace duckdb {

using SourcePtr = duckdb::unique_ptr<DuckportSource, decltype(&duckport_source_destroy)>;

class SourceFactory final : public ArrowScanFactory {
public:
	explicit SourceFactory(SourcePtr source);
	void GetSchema(ArrowSchema &schema) override;
	unique_ptr<ArrowArrayStreamWrapper> ProduceStream(ArrowStreamParameters &parameters) override;

	DuckportReadMetrics Snapshot() const;
	void ReportBytes(ClientContext &context);
	uint64_t ReportRows(uint64_t rows);

private:
	std::atomic<uint64_t> reported_bytes {0};
	std::atomic<uint64_t> reported_rows {0};
	SourcePtr source;
};

} // namespace duckdb
