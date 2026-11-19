#pragma once

#include "duckdb/function/function.hpp"
#include "source/options.hpp"

namespace duckdb {
inline void AddAttachmentOptions(TypedKwargs &options) {
	for (const auto *name : {"file_name", "format", "record_path", "endpoint"}) {
		options.Add(name, LogicalType::VARCHAR);
	}
}

// Own the common strings until the synchronous reader FFI call returns.
// Moving/copying this owner would invalidate the pointers already bound to options.
class AttachmentOptionsFfiView {
public:
	template <class Options>
	AttachmentOptionsFfiView(const TableFunctionBindInput &input, Options &options)
	    : file_name(ReadStringOption(input, "file_name")), format(ReadStringOption(input, "format")),
	      record_path(ReadStringOption(input, "record_path")), endpoint(ReadStringOption(input, "endpoint")) {
		options.file_name = file_name ? file_name->c_str() : nullptr;
		options.format = format ? format->c_str() : nullptr;
		options.record_path = record_path ? record_path->c_str() : nullptr;
		options.endpoint = endpoint ? endpoint->c_str() : nullptr;
	}
	AttachmentOptionsFfiView(const AttachmentOptionsFfiView &) = delete;
	AttachmentOptionsFfiView &operator=(const AttachmentOptionsFfiView &) = delete;

	const optional<string> &Endpoint() const & {
		return endpoint;
	}
	const optional<string> &Endpoint() const && = delete;

private:
	optional<string> file_name;
	optional<string> format;
	optional<string> record_path;
	optional<string> endpoint;
};
} // namespace duckdb
