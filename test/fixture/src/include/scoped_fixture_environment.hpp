#pragma once

#include "duckdb/common/string.hpp"

namespace duckdb {
// Restore each process setting when its fixture shuts down, including on error.
class ScopedFixtureEnvironment {
public:
	ScopedFixtureEnvironment(const char *name, const char *value);
	~ScopedFixtureEnvironment();
	ScopedFixtureEnvironment(const ScopedFixtureEnvironment &) = delete;
	ScopedFixtureEnvironment &operator=(const ScopedFixtureEnvironment &) = delete;

private:
	string name;
	string previous;
	bool was_set;
};

} // namespace duckdb
