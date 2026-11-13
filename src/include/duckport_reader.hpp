#pragma once
#include "duckdb.hpp"
#include <functional>

namespace duckdb {
void RegisterReader(ExtensionLoader &loader, const string &reader, const std::function<void(TypedKwargs &)> &configure,
                    vector<string> required);
}
