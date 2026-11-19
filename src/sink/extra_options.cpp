#include "sink/copy_query.hpp"
#include "duckdb/common/string_util.hpp"
#include "duckdb/parser/expression/constant_expression.hpp"
#include "duckdb/parser/expression/function_expression.hpp"
#include "duckdb/parser/parsed_data/copy_info.hpp"
#include "duckdb/parser/parsed_expression_iterator.hpp"
#include "duckdb/parser/tableref/table_function_ref.hpp"

namespace duckdb {
namespace {
vector<pair<Identifier, Value>> ReadExtraOptions(const vector<Value> &values) {
	if (values.size() != 1 || values[0].IsNull()) {
		throw BinderException("EXTRA_OPTIONS requires one map of reader option names to values");
	}
	const auto &value = values[0];
	vector<pair<Identifier, Value>> options;
	if (value.type().id() == LogicalTypeId::STRUCT) {
		const auto &names = StructType::GetChildTypes(value.type());
		const auto &children = StructValue::GetChildren(value);
		for (idx_t i = 0; i < names.size(); i++) {
			options.emplace_back(names[i].first, children[i]);
		}
	} else if (value.type().id() == LogicalTypeId::MAP) {
		for (const auto &item : MapValue::GetChildren(value)) {
			const auto &entry = StructValue::GetChildren(item);
			if (entry[0].IsNull() || entry[0].type().id() != LogicalTypeId::VARCHAR) {
				throw BinderException("EXTRA_OPTIONS names must be strings");
			}
			options.emplace_back(Identifier(entry[0].GetValue<string>()), entry[1]);
		}
	} else {
		throw BinderException("EXTRA_OPTIONS requires a map such as {'content_format': 'txt'}");
	}
	identifier_set_t names;
	for (const auto &option : options) {
		const auto &name = option.first.GetIdentifierName();
		if (name.empty() || name.find('\0') != string::npos || !names.insert(option.first).second ||
		    option.second.IsNull()) {
			throw BinderException("EXTRA_OPTIONS requires distinct nonempty names and non-NULL values");
		}
	}
	return options;
}
} // namespace

// Rewrite only explicit DuckPort reader calls. Their normal binders validate
// option names/types, and the COPY sink never sees source-specific options.
void ApplyCopyExtraOptions(QueryNode &query, CopyInfo &info) {
	auto entry = info.options.find(Identifier("extra_options"));
	if (entry == info.options.end()) {
		return;
	}
	auto options = ReadExtraOptions(entry->second);
	idx_t readers = 0;
	ParsedExpressionIterator::EnumerateQueryNodeChildren(
	    query, [](unique_ptr<ParsedExpression> &) {},
	    [&](TableRef &ref) {
		    if (ref.type != TableReferenceType::TABLE_FUNCTION) {
			    return;
		    }
		    auto &expression = *ref.Cast<TableFunctionRef>().function;
		    if (expression.GetExpressionClass() != ExpressionClass::FUNCTION) {
			    return;
		    }
		    auto &function = expression.Cast<FunctionExpression>();
		    if (!StringUtil::StartsWith(StringUtil::Lower(function.FunctionName().GetIdentifierName()),
		                                "duckport_scan_")) {
			    return;
		    }
		    readers++;
		    auto &arguments = function.GetArgumentsMutable();
		    for (const auto &option : options) {
			    for (const auto &argument : arguments) {
				    if (argument.HasName() && argument.GetName() == option.first) {
					    throw BinderException("EXTRA_OPTIONS duplicates reader option: %s", option.first);
				    }
			    }
			    arguments.emplace_back(option.first, ConstantExpression::FromValue(option.second));
		    }
	    });
	if (!options.empty() && readers == 0) {
		throw BinderException("EXTRA_OPTIONS requires an explicit duckport_scan_* source");
	}
	info.options.erase(entry);
}
} // namespace duckdb
