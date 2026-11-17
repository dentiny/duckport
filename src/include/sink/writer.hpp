#pragma once

#include "duckdb.hpp"
#include "duckdb/common/arrow/arrow.hpp"
#include "duckport.h"
#include <functional>

namespace duckdb {
enum class SinkWriteMode : uint8_t { CREATE, APPEND, OVERWRITE };

struct SinkWriteOptions {
	SinkWriteMode mode = SinkWriteMode::CREATE;
	vector<string> blob_columns;
	DuckportWriteOptions write_options {
	    .blob_inline_size_threshold = -1,
	    .blob_dedicated_size_threshold = -1,
	    .target_file_size = -1,
	};
	string catalog_uri;
	string warehouse;
	string storage_root;
};

// Backend boundary for a single fixed-schema session. Implementations must not
// publish from their destructor. The COPY state explicitly commits or aborts.
class SinkWriter {
public:
	virtual ~SinkWriter() = default;
	// Borrow the array synchronously or move it and clear its release callback.
	virtual void Write(ArrowArray &array) = 0;
	// Terminal even on failure; publication-failure cleanup belongs to the
	// backend.
	virtual void Commit() = 0;
	// Best-effort cleanup, including after a failed Write. Must not throw.
	virtual void Abort() noexcept = 0;
};

using SinkPtr = unique_ptr<DuckportSink, decltype(&duckport_sink_destroy)>;
unique_ptr<SinkWriter> MakeSinkWriter(SinkPtr writer);

// Inputs (including schema/credentials) are borrowed only during open. The
// backend must copy what it needs before returning its independently owned
// writer.
using WriterOpen = std::function<unique_ptr<SinkWriter>(const string &path, const DuckportStorageOptions &storage,
                                                        const SinkWriteOptions &options, ArrowSchema &schema)>;
} // namespace duckdb
