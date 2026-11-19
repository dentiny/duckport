#include "readers/pmc.h"
#include "source/options.hpp"
#include "source/readers/pmc.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenPmc(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
              const TableFunctionBindInput &input, DuckportSource **output) {
	auto format = ReadStringOption(input, "content_format");
	PmcOptions options {format ? format->c_str() : nullptr};
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_pmc_open(locator.c_str(), storage.config, scan, options, output);
}
char *OpenPmcShard(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                   const TableFunctionBindInput &input, DuckportSource **output) {
	auto manifest = ReadStringOption(input, "manifest");
	auto shard = ReadStringOption(input, "shard");
	auto format = ReadStringOption(input, "content_format");
	PmcShardOptions options {manifest ? manifest->c_str() : nullptr, shard ? shard->c_str() : nullptr,
	                         format ? format->c_str() : nullptr};
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_pmc_shard_open(locator.c_str(), storage.config, scan, options, output);
}
} // namespace duckdb
