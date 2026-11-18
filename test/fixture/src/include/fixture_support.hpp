#pragma once

#include "duckdb/common/string.hpp"
#include "duckdb/main/extension/extension_loader.hpp"

namespace duckdb {
string ReadFixtureFile(const string &provider, const string &name);

void RegisterBinaryFixtureCommand(ExtensionLoader &loader, const string &name,
                                  string (*command)(const string &, const string &));
void RegisterProviderFixtures(ExtensionLoader &loader);
void RegisterFileAndBlobFixtures(ExtensionLoader &loader);
} // namespace duckdb
