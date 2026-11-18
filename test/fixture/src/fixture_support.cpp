#include "fixture_support.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/function/scalar_function.hpp"
#include <fstream>
#include <iterator>

namespace duckdb {
string ReadFixtureFile(const string &provider, const string &name) {
	std::ifstream file(string(DUCKPORT_TESTDATA_DIR) + "/" + provider + "/" + name, std::ios::binary);
	if (!file) {
		throw IOException("Cannot open %s fixture: %s", provider, name);
	}
	return string(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
}

void RegisterBinaryFixtureCommand(ExtensionLoader &loader, const string &name,
                                  std::function<string(const string &, const string &)> command) {
	ScalarFunction function(
	    Identifier(name), {LogicalType::VARCHAR, LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [command](DataChunk &args, ExpressionState &, Vector &result) {
		    BinaryExecutor::Execute<string_t, string_t, string_t>(
		        args.data[0], args.data[1], result, args.size(), [&result, &command](string_t first, string_t second) {
			        return StringVector::AddString(result, command(first.GetString(), second.GetString()));
		        });
	    });
	function.SetVolatile();
	loader.RegisterFunction(function);
}
} // namespace duckdb
