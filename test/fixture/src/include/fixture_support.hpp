#pragma once

#include "duckdb/common/string.hpp"
#include "duckdb/main/extension/extension_loader.hpp"
#include <functional>

namespace duckdb {
string ReadFixtureFile(const string &provider, const string &name);

void RegisterBinaryFixtureCommand(ExtensionLoader &loader, const string &name,
                                  std::function<string(const string &, const string &)> command);
void RegisterProviderFixtures(ExtensionLoader &loader);
void RegisterFileAndBlobFixtures(ExtensionLoader &loader);
} // namespace duckdb
