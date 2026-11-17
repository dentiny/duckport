#pragma once
#include "sink/writer.hpp"

namespace duckdb {
void RegisterCopyFunction(ExtensionLoader &loader, const string &name, WriterOpen open);
unique_ptr<SinkWriter> OpenSinkWriter(const char *format, const string &path, const DuckportStorageOptions &storage,
                                     const SinkWriteOptions &options, ArrowSchema &schema);
void RegisterCopyFunctions(ExtensionLoader &loader);
} // namespace duckdb
