#pragma once

#include "duckdb/function/function.hpp"
#include "source/options.hpp"
#include "source/reader.hpp"
#include "storage_options.hpp"
#include <utility>

namespace duckdb {
inline void AddOaiOptions(TypedKwargs &options) {
	for (const auto *name : {"endpoint", "from_date", "until_date", "set_spec", "record_path"}) {
		options.Add(name, LogicalType::VARCHAR);
	}
	options.Add("max_pages", LogicalType::BIGINT);
	options.Add("max_records", LogicalType::BIGINT);
}

// Provider-specific FFI structs expose the same OAI option fields.
// Keep option strings and the credential snapshot alive through the FFI call.
template <class Options>
ReaderBind BindOaiReader(string default_endpoint,
                         char *(*open)(const char *, const DuckportStorageOptions &, const DuckportScanOptions &,
                                       const Options &, DuckportSource **)) {
	return [default_endpoint = std::move(default_endpoint),
	        open](ClientContext &context, const string &locator, const DuckportScanOptions &scan,
	              const TableFunctionBindInput &input, DuckportSource **output) {
		Options reader_options {};
		auto endpoint = ReadStringOption(input, "endpoint");
		reader_options.endpoint = endpoint ? endpoint->c_str() : nullptr;
		auto from_date = ReadStringOption(input, "from_date");
		reader_options.from_date = from_date ? from_date->c_str() : nullptr;
		auto until_date = ReadStringOption(input, "until_date");
		reader_options.until_date = until_date ? until_date->c_str() : nullptr;
		auto set_spec = ReadStringOption(input, "set_spec");
		reader_options.set_spec = set_spec ? set_spec->c_str() : nullptr;
		auto record_path = ReadStringOption(input, "record_path");
		reader_options.record_path = record_path ? record_path->c_str() : nullptr;
		ReadPaginationOptions(input, reader_options.max_pages, reader_options.max_records);
		StorageParameters storage(context, endpoint.value_or(default_endpoint) + "/", /*secret_type=*/ {});
		return open(locator.c_str(), storage.config, scan, reader_options, output);
	};
}
} // namespace duckdb
