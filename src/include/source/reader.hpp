#pragma once

#include "duckdb.hpp"
#include "duckport.h"
#include <functional>

namespace duckdb {
class LogicalOperator;
// Configure DuckPort scans before execution. Only a direct scan/projection can supply a size hint.
optional<uint64_t> ConfigureCopySources(LogicalOperator &plan, idx_t read_concurrency, bool inspect_size,
                                        bool automatic_reads);
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
