#pragma once
#include "sink/writer.hpp"

namespace duckdb {
void RegisterCopyFunction(ExtensionLoader &loader, const string &name, WriterOpen open);
void RegisterCopyFunctions(ExtensionLoader &loader);
} // namespace duckdb
