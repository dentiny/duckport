#pragma once

#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/function/table/arrow.hpp"
#include "duckport.h"

namespace duckdb {

using SourcePtr = duckdb::unique_ptr<DuckportSource, decltype(&duckport_source_destroy)>;

class SourceFactory final : public ArrowScanFactory {
public:
	explicit SourceFactory(SourcePtr source);
	void GetSchema(ArrowSchema &schema) override;
	unique_ptr<ArrowArrayStreamWrapper> ProduceStream(ArrowStreamParameters &parameters) override;

private:
	SourcePtr source;
};

} // namespace duckdb
