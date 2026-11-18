#pragma once

#include "duckdb.hpp"
#include "duckdb/common/arrow/arrow.hpp"
#include "duckdb/common/optional.hpp"
#include "duckport.h"
#include <functional>
#include <tuple>

namespace duckdb {
class WriterOpenContext;
enum class SinkWriteMode : uint8_t { CREATE, APPEND, OVERWRITE };

struct ColumnEncodingOptions {
	string column;
	string compression;
	bool has_compression_level = false;
	int32_t compression_level = 0;
	string structural_encoding;

	bool operator==(const ColumnEncodingOptions &other) const {
		return std::tie(column, compression, has_compression_level, compression_level, structural_encoding) ==
		       std::tie(other.column, other.compression, other.has_compression_level, other.compression_level,
		                other.structural_encoding);
	}
};

struct IndexOptions {
	string column;
	string option;

	bool operator==(const IndexOptions &other) const {
		return column == other.column && option == other.option;
	}
};

// Plan-owned configuration. Runtime credentials belong to WriterOpenContext.
struct SinkWriteOptions {
	static constexpr int64_t DEFAULT_TARGET_FILE_SIZE = 512LL * 1024 * 1024;

	SinkWriteMode mode = SinkWriteMode::CREATE;
	bool preserve_order = false;
	// Omission uses one writer; an unset count explicitly requests auto.
	// read_concurrency_set distinguishes inheritance from an explicit auto override.
	optional<int64_t> write_concurrency = 1;
	optional<int64_t> read_concurrency;
	bool read_concurrency_set = false;
	vector<string> blob_columns;
	DuckportWriteOptions write_options {
	    .blob_inline_size_threshold = -1,
	    .blob_dedicated_size_threshold = -1,
	    .target_file_size = DEFAULT_TARGET_FILE_SIZE,
	    .storage_version = nullptr,
	    .column_encodings = nullptr,
	    .column_encoding_count = 0,
	    .indexes = nullptr,
	    .index_count = 0,
	    .writer_count = 1,
	    .source_bytes = 0,
	    .source_bytes_known = false,
	};
	string catalog_uri;
	string warehouse;
	string storage_root;
	string storage_version;
	vector<ColumnEncodingOptions> column_encodings;
	vector<IndexOptions> indexes;

	DuckportWriteOptions ToFfi(vector<DuckportColumnEncoding> &encodings,
	                           vector<DuckportIndexOptions> &index_options) const {
		encodings.reserve(column_encodings.size());
		for (const auto &entry : column_encodings) {
			encodings.push_back(DuckportColumnEncoding {
			    .column = entry.column.c_str(),
			    .compression = entry.compression.empty() ? nullptr : entry.compression.c_str(),
			    .has_compression_level = entry.has_compression_level,
			    .compression_level = entry.compression_level,
			    .structural_encoding = entry.structural_encoding.empty() ? nullptr : entry.structural_encoding.c_str(),
			});
		}
		index_options.reserve(indexes.size());
		for (const auto &entry : indexes) {
			index_options.push_back(DuckportIndexOptions {
			    .column = entry.column.c_str(),
			    .option = entry.option.c_str(),
			});
		}
		auto result = write_options;
		result.indexes = index_options.data();
		result.index_count = index_options.size();
		result.column_encodings = encodings.data();
		result.column_encoding_count = encodings.size();
		result.storage_version = storage_version.empty() ? nullptr : storage_version.c_str();
		return result;
	}
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
	virtual DuckportWriteMetrics Metrics() const = 0;
	// Best-effort cleanup, including after a failed Write. Must not throw.
	virtual void Abort() noexcept = 0;
};

using SinkPtr = unique_ptr<DuckportSink, decltype(&duckport_sink_destroy)>;
unique_ptr<SinkWriter> MakeSinkWriter(SinkPtr writer);

// Inputs (including schema/credentials) are borrowed only during open. The
// backend must copy what it needs before returning its independently owned
// writer.
using WriterOpen = std::function<unique_ptr<SinkWriter>(const string &path, WriterOpenContext &context,
                                                        const SinkWriteOptions &options, ArrowSchema &schema)>;
} // namespace duckdb
