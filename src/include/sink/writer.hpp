#pragma once

#include "duckdb.hpp"
#include "duckdb/common/arrow/arrow.hpp"
#include "duckport.h"
#include <functional>

namespace duckdb {
enum class SinkWriteMode : uint8_t { CREATE, APPEND, OVERWRITE };

struct SinkWriteOptions {
	SinkWriteMode mode = SinkWriteMode::CREATE;
};

// Backend boundary for a single fixed-schema session. Implementations must not
// publish from their destructor. The COPY state explicitly commits or aborts.
class SinkWriter {
public:
	virtual ~SinkWriter() = default;
	// Borrow the array synchronously or move it and clear its release callback.
	virtual void Write(ArrowArray &array) = 0;
	// Terminal even on failure; publication-failure cleanup belongs to the backend.
	virtual void Commit() = 0;
	// Best-effort cleanup, including after a failed Write. Must not throw.
	virtual void Abort() noexcept = 0;
};

// Inputs (including schema/credentials) are borrowed only during open. The
// backend must copy what it needs before returning its independently owned writer.
using WriterOpen = std::function<unique_ptr<SinkWriter>(const string &path, const DuckportStorageOptions &storage,
                                                        const SinkWriteOptions &options, ArrowSchema &schema)>;
} // namespace duckdb
