#define DUCKDB_EXTENSION_MAIN
#include "duckport_test_fixture_extension.hpp"
#include "fixture_support.hpp"
#include "iceberg.hpp"

namespace duckdb {
namespace {
void LoadInternal(ExtensionLoader &loader) {
	RegisterIcebergFixture(loader);
	RegisterFileAndBlobFixtures(loader);
	RegisterProviderFixtures(loader);
}
} // namespace

void DuckportTestFixtureExtension::Load(ExtensionLoader &loader) {
	LoadInternal(loader);
}
string DuckportTestFixtureExtension::Name() {
	return "duckport_test_fixture";
}
string DuckportTestFixtureExtension::Version() const {
	return "";
}
} // namespace duckdb

extern "C" {
DUCKDB_CPP_EXTENSION_ENTRY(duckport_test_fixture, loader) {
	duckdb::LoadInternal(loader);
}
}
