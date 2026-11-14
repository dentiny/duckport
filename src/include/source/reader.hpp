#pragma once

#include "duckdb.hpp"
#include "duckport.h"
#include <functional>

namespace duckdb {
using ReaderOpen = char *(*)(const char *, const ::StorageOptions *, const ::ScanOptions *, DuckportSource **);
using ReaderBind = std::function<char *(ClientContext &, const string &, const ::ScanOptions *,
										const TableFunctionBindInput &, DuckportSource **)>;
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
					const std::function<void(TypedKwargs &)> &configure = {});
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderBind open,
					const std::function<void(TypedKwargs &)> &configure = {});
} // namespace duckdb
