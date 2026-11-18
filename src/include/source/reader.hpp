#pragma once

#include "duckdb.hpp"
#include "duckport.h"
#include <functional>

namespace duckdb {
using ReaderOpen = char *(*)(const char *, const DuckportStorageOptions &, const DuckportScanOptions &,
                             DuckportSource **);
using ReaderBind = std::function<char *(ClientContext &, const string &, const DuckportScanOptions &,
                                        const TableFunctionBindInput &, DuckportSource **)>;
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
                    const std::function<void(TypedKwargs &)> &add_reader_options = {});
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderBind open,
                    const std::function<void(TypedKwargs &)> &add_reader_options = {});
void RegisterFileReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
                        const std::function<void(TypedKwargs &)> &add_reader_options = {});
void RegisterFileReader(ExtensionLoader &loader, const string &name, ReaderBind open,
                        const std::function<void(TypedKwargs &)> &add_reader_options = {});
} // namespace duckdb
