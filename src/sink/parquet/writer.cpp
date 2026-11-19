#include "sink/copy.hpp"
#include "sink/write_options_ffi.hpp"
#include "sink/writer_context.hpp"
#include "error.hpp"

namespace duckdb {
unique_ptr<SinkWriter> OpenParquetWriter(const string &path, WriterOpenContext &context,
                                         const SinkWriteOptions &options, ArrowSchema &schema) {
	SinkWriteOptionsFfiView write_options(options);
	DuckportSink *raw = nullptr;
	CheckError(duckport_parquet_open(path.c_str(), &schema, write_options.Mode(), options.blob_columns.size(),
	                                 &write_options.Get(), &context.storage, &raw));
	return MakeSinkWriter(SinkPtr(raw, duckport_sink_destroy));
}
} // namespace duckdb
