#pragma once

#include "duckdb.hpp"
#include "duckport.h"
#include <functional>

namespace duckdb {
using ReaderOpen = char *(*)(const char *, const ::StorageOptions *, const ::ScanOptions *, DuckportSource **);
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
					const std::function<void(TypedKwargs &)> &configure = {}, const string &secret_type = {});
} // namespace duckdb
