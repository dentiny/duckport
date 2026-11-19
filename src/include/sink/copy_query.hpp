#pragma once

#include "duckdb.hpp"
#include "duckdb/parser/parsed_data/sample_options.hpp"
#include "duckdb/parser/parsed_expression.hpp"
#include "duckdb/parser/query_node.hpp"

namespace duckdb {
struct CopyInfo;
struct BoundStatement;

void ApplyCopyExtraOptions(QueryNode &query, CopyInfo &info);
unique_ptr<SampleOptions> TakeSampleOptions(ClientContext &context, CopyInfo &info);
vector<unique_ptr<ParsedExpression>> TakeGeneratedColumns(ClientContext &context, CopyInfo &info);
unique_ptr<QueryNode> ApplyCopySample(unique_ptr<QueryNode> query, unique_ptr<SampleOptions> sample);
unique_ptr<QueryNode> ApplyGeneratedColumns(unique_ptr<QueryNode> query,
                                            const vector<unique_ptr<ParsedExpression>> &columns);
void ValidateBoundGeneratedColumns(const BoundStatement &source, const vector<unique_ptr<ParsedExpression>> &columns);
} // namespace duckdb
