#include "provider_fixture.hpp"
#include "fixture_support.hpp"
#include "duckdb/common/helper.hpp"
#include "duckdb/common/vector_operations/unary_executor.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/function/table_function.hpp"

namespace duckdb {
namespace {
struct RequestsBindData : public FunctionData {
	string provider;
	unique_ptr<FunctionData> Copy() const override {
		auto result = make_uniq<RequestsBindData>();
		result->provider = provider;
		return std::move(result);
	}
	bool Equals(const FunctionData &other_p) const override {
		return this == &other_p;
	}
};

unique_ptr<FunctionData> RequestsBind(ClientContext &, TableFunctionBindInput &input, vector<LogicalType> &types,
                                      vector<Identifier> &names) {
	auto result = make_uniq<RequestsBindData>();
	if (input.inputs[0].IsNull()) {
		throw InvalidInputException("Fixture provider cannot be NULL");
	}
	result->provider = input.inputs[0].GetValue<string>();
	for (const auto &name : {"origin", "target", "authorization", "api_key", "headers", "request_payer"}) {
		names.emplace_back(name);
		types.emplace_back(LogicalType::VARCHAR);
	}
	for (const auto &name : {"is_download", "has_authorization", "has_request_payer"}) {
		names.emplace_back(name);
		types.emplace_back(LogicalType::BOOLEAN);
	}
	return std::move(result);
}

struct RequestsState : public GlobalTableFunctionState {
	vector<FixtureRequest> traces;
	idx_t offset = 0;
};

unique_ptr<GlobalTableFunctionState> RequestsInit(ClientContext &, TableFunctionInitInput &input) {
	auto &bind = input.bind_data->Cast<RequestsBindData>();
	auto result = make_uniq<RequestsState>();
	result->traces = ProviderFixtureRequests(bind.provider);
	return std::move(result);
}

void RequestsScan(ClientContext &, TableFunctionInput &input, DataChunk &output) {
	auto &state = input.global_state->Cast<RequestsState>();
	idx_t count = 0;
	while (state.offset < state.traces.size() && count < STANDARD_VECTOR_SIZE) {
		const auto &row = state.traces[state.offset++];
		output.data[0].Append(Value(row.origin));
		output.data[1].Append(Value(row.target));
		output.data[2].Append(Value(row.authorization));
		output.data[3].Append(Value(row.api_key));
		output.data[4].Append(Value(row.headers));
		output.data[5].Append(Value(row.request_payer));
		output.data[6].Append(Value::BOOLEAN(row.is_download));
		output.data[7].Append(Value::BOOLEAN(row.has_authorization));
		output.data[8].Append(Value::BOOLEAN(row.has_request_payer));
		count++;
	}
	output.CheckCardinality(count);
}

} // namespace

void RegisterProviderFixtures(ExtensionLoader &loader) {
	RegisterBinaryFixtureCommand(loader, "duckport_fixture", ProviderFixtureCommand);
	ScalarFunction ia(
	    "duckport_ia_fixture", {LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [](DataChunk &args, ExpressionState &, Vector &result) {
		    UnaryExecutor::Execute<string_t, string_t>(args.data[0], result, args.size(), [&result](string_t input) {
			    return StringVector::AddString(result, ProviderFixtureCommand("internet_archive", input.GetString()));
		    });
	    });
	ia.SetVolatile();
	loader.RegisterFunction(ia);
	TableFunction requests("duckport_fixture_requests", {LogicalType::VARCHAR}, RequestsScan, RequestsBind,
	                       RequestsInit);
	loader.RegisterFunction(requests);
}
} // namespace duckdb
