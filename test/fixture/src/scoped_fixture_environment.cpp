#include "scoped_fixture_environment.hpp"
#include <cstdlib>

namespace duckdb {
namespace {
void SetFixtureEnvironment(const char *name, const char *value) {
#ifdef _WIN32
	_putenv_s(name, value ? value : "");
#else
	if (value) {
		setenv(name, value, 1);
	} else {
		unsetenv(name);
	}
#endif
}
} // namespace

ScopedFixtureEnvironment::ScopedFixtureEnvironment(const char *name, const char *value) : name(name) {
	auto old = std::getenv(name);
	was_set = old != nullptr;
	if (old) {
		previous = old;
	}
	SetFixtureEnvironment(name, value);
}

ScopedFixtureEnvironment::~ScopedFixtureEnvironment() {
	SetFixtureEnvironment(name.c_str(), was_set ? previous.c_str() : nullptr);
}

} // namespace duckdb
