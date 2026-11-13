#pragma once

#include "duckport.h"
#include "duckdb/function/table/arrow.hpp"
#include <memory>

namespace duckdb {

using SourcePtr = std::unique_ptr<DuckportSource, decltype(&duckport_source_destroy)>;

class SourceFactory final : public ArrowScanFactory {
public:
	explicit SourceFactory(SourcePtr source);
	void GetSchema(ArrowSchema &schema) override;
	unique_ptr<ArrowArrayStreamWrapper> ProduceStream(ArrowStreamParameters &parameters) override;

private:
	SourcePtr source;
};

} // namespace duckdb
