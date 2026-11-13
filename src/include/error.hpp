#pragma once

#include "duckport.h"
#include "duckdb/common/exception.hpp"
#include <memory>

namespace duckdb {

inline void CheckError(char *error) {
	std::unique_ptr<char, decltype(&duckport_source_error_free)> owned(error, duckport_source_error_free);
	if (owned) {
		throw IOException("DuckPort: %s", owned.get());
	}
}

} // namespace duckdb
