#include "source/reader.hpp"
#include "duckdb/common/string_util.hpp"
#include "duckdb/function/table/arrow.hpp"
#include "duckdb/main/settings.hpp"
#include "error.hpp"
#include "source/source_factory.hpp"

namespace duckdb {
namespace {
struct ReaderInfo : TableFunctionInfo {
	explicit ReaderInfo(ReaderOpen open) : open(open) {}
	ReaderOpen open;
};
void CheckAccess(ClientContext &context) {
	if (!Settings::Get<EnableExternalAccessSetting>(context)) {
		throw PermissionException("DuckPort requires enable_external_access");
	}
}
string ReadString(const Value &value) {
	if (value.IsNull()) {
		throw BinderException("DuckPort arguments cannot be NULL");
	}
	auto text = value.ToString();
	if (text.find('\0') != string::npos) {
		throw BinderException("DuckPort arguments cannot contain NUL bytes");
	}
	return text;
}
unique_ptr<FunctionData> BindReader(ClientContext &context, TableFunctionBindInput &input, vector<LogicalType> &types,
									vector<Identifier> &names) {
	CheckAccess(context);
	auto path = ReadString(input.inputs[0]);
	constexpr idx_t DEFAULT_BATCH_SIZE = 8192;
	idx_t batch_size = DEFAULT_BATCH_SIZE;
	unordered_map<string, string> options;
	for (const auto &entry : input.named_parameters) {
		if (entry.second.IsNull()) {
			throw BinderException("DuckPort arguments cannot be NULL");
		}
		auto key = entry.first.GetIdentifierName();
		if (key == "batch_size") {
			batch_size = idx_t(BigIntValue::Get(entry.second));
		} else {
			options[key] = ReadString(entry.second);
		}
	}
	auto json = StringUtil::ToJSONMap(options);
	DuckportSource *raw = nullptr;
	CheckError(input.info->Cast<ReaderInfo>().open(path.c_str(), json.c_str(), batch_size, &raw));
	auto factory = make_shared_ptr<SourceFactory>(SourcePtr(raw, duckport_source_destroy));
	auto result = make_uniq<ArrowScanFunctionData>(std::move(factory));
	result->factory->GetSchema(result->schema_root.arrow_schema);
	ArrowTableFunction::PopulateArrowTableSchema(context, result->arrow_table, result->schema_root.arrow_schema);
	names = StringsToIdentifiers(result->arrow_table.GetNames());
	types = result->arrow_table.GetTypes();
	if (types.empty()) {
		throw BinderException("source must have at least one column");
	}
	result->all_types = types;
	result->projection_pushdown_enabled = false;
	return std::move(result);
}
unique_ptr<GlobalTableFunctionState> InitReader(ClientContext &context, TableFunctionInitInput &input) {
	CheckAccess(context);
	return ArrowTableFunction::ArrowScanInitGlobal(context, input);
}
} // namespace
void RegisterReader(ExtensionLoader &loader, const string &name, ReaderOpen open,
					const std::function<void(TypedKwargs &)> &configure) {
	TableFunction function(Identifier(name), {LogicalType::VARCHAR}, ArrowTableFunction::ArrowScanFunction, BindReader,
						   InitReader, ArrowTableFunction::ArrowScanInitLocal);
	function.function_info = make_shared_ptr<ReaderInfo>(open);
	function.GetSignature().WithTypedKwargs("options", [&](TypedKwargs &options) {
		options.Add("batch_size", LogicalType::BIGINT);
		configure(options);
	});
	loader.RegisterFunction(function);
}
} // namespace duckdb
