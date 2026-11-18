#pragma once

#include "sink/writer.hpp"

namespace duckdb {
struct CopyInfo;
struct CopyOptionsInput;

void CheckCopyOptionError(char *error);
SinkWriteOptions ParseCopyOptions(ClientContext &context, const CopyInfo &info);
void RegisterCopyOptions(ClientContext &context, CopyOptionsInput &input);
} // namespace duckdb
