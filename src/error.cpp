#include "duckdb/common/exception.hpp"
#include "duckport.h"
#include "error.hpp"
#include "duckdb/common/unique_ptr.hpp"

namespace duckdb {

void CheckError(char *error) {
	duckdb::unique_ptr<char, decltype(&duckport_source_error_free)> owned(error, duckport_source_error_free);
	if (owned) {
		throw IOException("DuckPort: %s", owned.get());
	}
}

} // namespace duckdb
