#pragma once
#include "sink/writer.hpp"

namespace duckdb {
void RegisterCopyFunction(ExtensionLoader &loader, const string &name, WriterOpen open);
unique_ptr<SinkWriter> OpenLanceWriter(const string &path, const SinkStorageOptions &storage,
                                       const SinkWriteOptions &options, ArrowSchema &schema);
unique_ptr<SinkWriter> OpenIcebergWriter(const string &path, const SinkStorageOptions &storage,
                                         const SinkWriteOptions &options, ArrowSchema &schema);
unique_ptr<SinkWriter> OpenParquetWriter(const string &path, const SinkStorageOptions &storage,
                                         const SinkWriteOptions &options, ArrowSchema &schema);
void RegisterCopyFunctions(ExtensionLoader &loader);
} // namespace duckdb
