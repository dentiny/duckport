#pragma once

#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/function/table/arrow.hpp"
#include "duckport.h"
#include "duckdb/common/optional.hpp"
#include "duckdb/common/atomic.hpp"

namespace duckdb {

using SourcePtr = duckdb::unique_ptr<DuckportSource, decltype(&duckport_source_destroy)>;

class SourceFactory final : public ArrowScanFactory {
public:
	explicit SourceFactory(SourcePtr source, idx_t read_concurrency, bool automatic);
	string ReadConcurrencyDescription() const;
	void GetSchema(ArrowSchema &schema) override;
	unique_ptr<ArrowArrayStreamWrapper> ProduceStream(ArrowStreamParameters &parameters) override;

	DuckportReadMetrics Snapshot() const;
	optional<uint64_t> ConfigureCopy(idx_t read_concurrency, bool inspect_size, bool automatic);
	void ReportBytes(ClientContext &context);
	uint64_t ReportRows(uint64_t rows);

private:
	atomic<uint64_t> reported_bytes {0};
	atomic<uint64_t> reported_rows {0};
	SourcePtr source;
	idx_t read_concurrency;
	bool automatic_reads;
};

} // namespace duckdb
