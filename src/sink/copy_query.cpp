#include "sink/copy_query.hpp"
#include "sink/copy_options.hpp"
#include "duckport.h"
#include "duckdb/parser/parsed_data/copy_info.hpp"
#include "duckdb/parser/expression/star_expression.hpp"
#include "duckdb/parser/parsed_data/sample_options.hpp"
#include "duckdb/parser/parsed_expression_iterator.hpp"
#include "duckdb/parser/parser.hpp"
#include "duckdb/parser/query_node/select_node.hpp"
#include "duckdb/parser/statement/select_statement.hpp"
#include "duckdb/parser/tableref/subqueryref.hpp"
#include "duckdb/planner/bound_statement.hpp"
#include "duckdb/planner/expression/bound_columnref_expression.hpp"
#include "duckdb/planner/expression_iterator.hpp"
#include "duckdb/planner/logical_operator.hpp"

namespace duckdb {
unique_ptr<SampleOptions> TakeSampleOptions(ClientContext &context, CopyInfo &info) {
	auto percent = info.options.find(Identifier("sample_percent"));
	auto rows = info.options.find(Identifier("sample_rows"));
	if (percent != info.options.end() && rows != info.options.end()) {
		throw BinderException("Only one of SAMPLE_PERCENT or SAMPLE_ROWS can be specified");
	}
	auto entry = percent != info.options.end() ? percent : rows;
	if (entry == info.options.end()) {
		return nullptr;
	}
	bool is_percent = percent != info.options.end();
	const auto &values = entry->second;
	if (values.size() != 1 || values[0].IsNull() ||
	    (is_percent ? !values[0].type().IsNumeric() : !values[0].type().IsIntegral())) {
		throw BinderException("%s requires one %s", entry->first, is_percent ? "number" : "integer");
	}
	auto sample = make_uniq<SampleOptions>();
	sample->is_percentage = is_percent;
	sample->method = is_percent ? SampleMethod::BERNOULLI_SAMPLE : SampleMethod::RESERVOIR_SAMPLE;
	sample->sample_size = values[0].CastAs(context, is_percent ? LogicalType::DOUBLE : LogicalType::BIGINT);
	CheckCopyOptionError(is_percent ? duckport_validate_sample_percent(sample->sample_size.GetValue<double>())
	                                : duckport_validate_sample_rows(sample->sample_size.GetValue<int64_t>(),
	                                                                SampleOptions::MAX_SAMPLE_ROWS));
	info.options.erase(entry);
	return sample;
}

namespace {
void ValidateGeneratedExpression(const Identifier &column, const ParsedExpression &expression) {
	switch (expression.GetExpressionClass()) {
	case ExpressionClass::STAR:
		throw BinderException("GENERATED_COLUMNS expression cannot contain *: %s", column);
	case ExpressionClass::PARAMETER:
		throw BinderException("GENERATED_COLUMNS expression cannot contain a parameter: %s", column);
	default:
		break;
	}
	ParsedExpressionIterator::EnumerateChildren(
	    expression, [&](const ParsedExpression &child) { ValidateGeneratedExpression(column, child); });
}

unique_ptr<ParsedExpression> ParseGeneratedExpression(Parser &parser, const Identifier &name, const Value &value) {
	if (value.IsNull() || value.type().id() != LogicalTypeId::VARCHAR ||
	    StringValue::Get(value).find('\0') != string::npos) {
		throw BinderException("GENERATED_COLUMNS expression must be text without NUL bytes: %s", name);
	}
	auto expressions = parser.ParseExpressionList(value.GetValue<string>());
	if (expressions.size() != 1) {
		throw BinderException("GENERATED_COLUMNS expression must be exactly one SQL expression: %s", name);
	}
	ValidateGeneratedExpression(name, *expressions[0]);
	expressions[0]->SetAlias(name);
	return std::move(expressions[0]);
}

unique_ptr<SelectNode> SelectFrom(unique_ptr<QueryNode> query) {
	auto subquery = make_uniq<SelectStatement>();
	subquery->node = std::move(query);
	auto result = make_uniq<SelectNode>();
	result->select_list.push_back(make_uniq<StarExpression>());
	result->from_table = make_uniq<SubqueryRef>(std::move(subquery));
	return result;
}
} // namespace

unique_ptr<QueryNode> ApplyCopySample(unique_ptr<QueryNode> query, unique_ptr<SampleOptions> sample) {
	auto result = SelectFrom(std::move(query));
	result->sample = std::move(sample);
	return std::move(result);
}

vector<unique_ptr<ParsedExpression>> TakeGeneratedColumns(ClientContext &context, CopyInfo &info) {
	vector<unique_ptr<ParsedExpression>> result;
	auto entry = info.options.find(Identifier("generated_columns"));
	if (entry == info.options.end()) {
		return result;
	}
	const auto &values = entry->second;
	if (values.size() != 1 || values[0].type().id() != LogicalTypeId::STRUCT || values[0].IsNull()) {
		throw BinderException("GENERATED_COLUMNS requires a struct such as {'column': 'expression'}");
	}
	auto &entries = StructType::GetChildTypes(values[0].type());
	auto &children = StructValue::GetChildren(values[0]);
	identifier_set_t names;
	Parser parser(context);
	result.reserve(entries.size());
	for (idx_t i = 0; i < entries.size(); i++) {
		const auto &name = entries[i].first;
		if (name.GetIdentifierName().empty() || name.GetIdentifierName().find('\0') != string::npos ||
		    !names.insert(name).second) {
			throw BinderException("GENERATED_COLUMNS requires unique, nonempty "
			                      "column names without NUL bytes");
		}
		result.emplace_back(ParseGeneratedExpression(parser, name, children[i]));
	}
	info.options.erase(entry);
	return result;
}

unique_ptr<QueryNode> ApplyGeneratedColumns(unique_ptr<QueryNode> query,
                                            const vector<unique_ptr<ParsedExpression>> &columns) {
	auto result = SelectFrom(std::move(query));
	result->select_list.reserve(result->select_list.size() + columns.size());
	for (const auto &column : columns) {
		result->select_list.emplace_back(column->Copy());
	}
	return std::move(result);
}

// Non-scalar operations, including those introduced by macros, bind references
// to operators other than the source row.
void ValidateBoundGeneratedColumns(const BoundStatement &source, const vector<unique_ptr<ParsedExpression>> &columns) {
	auto &expressions = source.plan->expressions;
	if (source.plan->type != LogicalOperatorType::LOGICAL_PROJECTION || expressions.size() != source.names.size() ||
	    expressions[0]->GetExpressionClass() != ExpressionClass::BOUND_COLUMN_REF) {
		throw InternalException("GENERATED_COLUMNS expected a projection over the COPY source");
	}
	auto source_table = expressions[0]->Cast<BoundColumnRefExpression>().Binding().table_index;
	auto first = source.names.size() - columns.size();
	for (idx_t i = 0; i < columns.size(); i++) {
		const auto &name = columns[i]->GetAlias();
		if (source.names[first + i] != name) {
			throw BinderException("GENERATED_COLUMNS column already exists in the COPY source: %s", name);
		}
		auto &expression = *expressions[first + i];
		if (expression.IsVolatile()) {
			throw BinderException("GENERATED_COLUMNS expression cannot use volatile functions: %s", name);
		}
		ExpressionIterator::VisitExpression<BoundColumnRefExpression>(
		    expression, [&](const BoundColumnRefExpression &reference) {
			    if (reference.Binding().table_index != source_table) {
				    throw BinderException("GENERATED_COLUMNS expression must be computed from one source "
				                          "row (no aggregate, window, "
				                          "unnest or subquery): %s",
				                          name);
			    }
		    });
	}
}

} // namespace duckdb
