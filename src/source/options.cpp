#include "source/options.hpp"
#include "duckdb/common/string_util.hpp"
#include "duckdb/parallel/task_scheduler.hpp"

namespace duckdb {
optional<string> ReadStringOption(const TableFunctionBindInput &input, const char *name) {
	auto entry = input.named_parameters.find(name);
	if (entry == input.named_parameters.end()) {
		return nullopt;
	}
	return ReadSourceString(entry->second);
}
void ReadBooleanOption(const TableFunctionBindInput &input, const char *name, int32_t &output) {
	auto entry = input.named_parameters.find(name);
	if (entry != input.named_parameters.end()) {
		output = BooleanValue::Get(entry->second) ? 1 : 0;
	}
}
void ReadPaginationOptions(const TableFunctionBindInput &input, int64_t &max_pages, int64_t &max_records) {
	for (const auto &name : {"max_pages", "max_records"}) {
		auto entry = input.named_parameters.find(name);
		if (entry == input.named_parameters.end()) {
			continue;
		}
		auto value = BigIntValue::Get(entry->second);
		if (value <= 0) {
			throw BinderException("%s must be positive", name);
		}
		(string(name) == "max_pages" ? max_pages : max_records) = value;
	}
}
string ReadSourceString(const Value &value) {
	if (value.IsNull()) {
		throw BinderException("DuckPort arguments cannot be NULL");
	}
	auto text = StringValue::Get(value);
	if (text.find('\0') != string::npos) {
		throw BinderException("DuckPort arguments cannot contain NUL bytes");
	}
	return text;
}

optional<int64_t> ReadConcurrency(const Identifier &name, const Value &value) {
	if (!value.IsNull()) {
		if (value.type().id() == LogicalTypeId::VARCHAR && StringUtil::CIEquals(StringValue::Get(value), "auto")) {
			return nullopt;
		}
		if (value.type().IsIntegral()) {
			auto number = value.DefaultTryCastAs(LogicalType::BIGINT);
			if (number && number->GetValue<int64_t>() > 0) {
				return number->GetValue<int64_t>();
			}
		}
	}
	throw BinderException("%s requires 'auto' or a positive integer that fits in BIGINT",
	                      StringUtil::Upper(name.GetIdentifierName()));
}

idx_t AutoReadConcurrency(ClientContext &context) {
	// Streaming queues are not byte bounded. Keep automatic read-ahead conservative.
	return MinValue<idx_t>(4, TaskScheduler::GetScheduler(context).NumberOfThreads());
}

DuckportScanOptions ReadScanOptions(ClientContext &context, const TableFunctionBindInput &input) {
	for (const auto &entry : input.named_parameters) {
		if (entry.second.IsNull()) {
			throw BinderException("DuckPort arguments cannot be NULL");
		}
	}
	constexpr idx_t DEFAULT_BATCH_SIZE = 8192;
	DuckportScanOptions scan {DEFAULT_BATCH_SIZE, AutoReadConcurrency(context), nullptr, 0};
	auto batch = input.named_parameters.find("batch_size");
	if (batch != input.named_parameters.end()) {
		scan.batch_size = idx_t(BigIntValue::Get(batch->second));
	}
	auto concurrency = input.named_parameters.find("read_concurrency");
	if (concurrency != input.named_parameters.end()) {
		scan.read_concurrency =
		    ReadConcurrency(Identifier("READ_CONCURRENCY"), concurrency->second).value_or(AutoReadConcurrency(context));
	}
	return scan;
}
} // namespace duckdb
