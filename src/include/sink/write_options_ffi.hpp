#pragma once

#include "duckdb/common/vector.hpp"
#include "duckport.h"

namespace duckdb {
struct SinkWriteOptions;

// Owns the ABI arrays and borrows strings from the plan-owned options, which
// must outlive this view. The backend copies all inputs during synchronous open.
class SinkWriteOptionsFfiView {
public:
	explicit SinkWriteOptionsFfiView(const SinkWriteOptions &options);
	SinkWriteOptionsFfiView(const SinkWriteOptionsFfiView &) = delete;
	SinkWriteOptionsFfiView &operator=(const SinkWriteOptionsFfiView &) = delete;

	const char *Mode() const;
	const DuckportWriteOptions &Get() const &;
	const DuckportWriteOptions &Get() const && = delete;

private:
	const char *mode;
	vector<DuckportColumnEncoding> encodings;
	vector<DuckportIndexOptions> index_options;
	DuckportWriteOptions config;
};
} // namespace duckdb
