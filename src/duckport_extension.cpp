#define DUCKDB_EXTENSION_MAIN

#include "duckport_extension.hpp"
#include "duckdb.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/function/scalar_function.hpp"
#include <duckdb/parser/parsed_data/create_scalar_function_info.hpp>

// OpenSSL linked through vcpkg
#include <openssl/opensslv.h>

namespace duckdb {

inline void DuckportScalarFun(DataChunk &args, ExpressionState &state, Vector &result) {
	auto &name_vector = args.data[0];
	UnaryExecutor::Execute<string_t, string_t>(name_vector, result, args.size(), [&](string_t name) {
		return StringVector::AddString(result, "...........🦆 " + name.GetString());
	});
}

inline void DuckportOpenSSLVersionScalarFun(DataChunk &args, ExpressionState &state, Vector &result) {
	auto &name_vector = args.data[0];
	UnaryExecutor::Execute<string_t, string_t>(name_vector, result, args.size(), [&](string_t name) {
		return StringVector::AddString(result, "Duckport " + name.GetString() + ", my linked OpenSSL version is " +
		                                           OPENSSL_VERSION_TEXT);
	});
}

static void LoadInternal(ExtensionLoader &loader) {
	// Register a scalar function
	auto duckport_scalar_function =
	    ScalarFunction("duckport", {LogicalType::VARCHAR}, LogicalType::VARCHAR, DuckportScalarFun);

	loader.RegisterFunction(duckport_scalar_function);

	// Register another scalar function
	auto duckport_openssl_version_scalar_function = ScalarFunction("duckport_openssl_version", {LogicalType::VARCHAR},
	                                                             LogicalType::VARCHAR, DuckportOpenSSLVersionScalarFun);
	loader.RegisterFunction(duckport_openssl_version_scalar_function);
}

void DuckportExtension::Load(ExtensionLoader &loader) {
	LoadInternal(loader);
}
std::string DuckportExtension::Name() {
	return "duckport";
}

std::string DuckportExtension::Version() const {
#ifdef EXT_VERSION_DUCKPORT
	return EXT_VERSION_DUCKPORT;
#else
	return "";
#endif
}

} // namespace duckdb

extern "C" {

DUCKDB_CPP_EXTENSION_ENTRY(duckport, loader) {
	duckdb::LoadInternal(loader);
}
}
