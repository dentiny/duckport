#include "sink/write_options_ffi.hpp"
#include "sink/writer.hpp"

namespace duckdb {
SinkWriteOptionsFfiView::SinkWriteOptionsFfiView(const SinkWriteOptions &options)
    : mode(options.mode == SinkWriteMode::APPEND      ? "append"
           : options.mode == SinkWriteMode::OVERWRITE ? "overwrite"
                                                      : "create"),
      config(options.write_options) {
	encodings.reserve(options.column_encodings.size());
	for (const auto &entry : options.column_encodings) {
		encodings.push_back(DuckportColumnEncoding {
		    .column = entry.column.c_str(),
		    .compression = entry.compression.empty() ? nullptr : entry.compression.c_str(),
		    .has_compression_level = entry.has_compression_level,
		    .compression_level = entry.compression_level,
		    .structural_encoding = entry.structural_encoding.empty() ? nullptr : entry.structural_encoding.c_str(),
		});
	}
	index_options.reserve(options.indexes.size());
	for (const auto &entry : options.indexes) {
		index_options.push_back(DuckportIndexOptions {
		    .column = entry.column.c_str(),
		    .option = entry.option.c_str(),
		});
	}
	config.indexes = index_options.data();
	config.index_count = index_options.size();
	config.column_encodings = encodings.data();
	config.column_encoding_count = encodings.size();
	config.storage_version = options.storage_version.empty() ? nullptr : options.storage_version.c_str();
}

const char *SinkWriteOptionsFfiView::Mode() const {
	return mode;
}

const DuckportWriteOptions &SinkWriteOptionsFfiView::Get() const & {
	return config;
}
} // namespace duckdb
