#include "duckdb.hpp"
#include "duckport.h"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/common/vector_operations/unary_executor.hpp"
#include "duckdb/common/vector_operations/ternary_executor.hpp"
#include "duckdb/main/extension/extension_loader.hpp"
#include <filesystem>
#include <fstream>

extern "C" {
char *duckport_fixture_security(const DuckportSecurityOptions *, char **);
char *duckport_fixture_blob_layout(const char *, const char *, char **);
char *duckport_fixture_field_metadata(const char *, const char *, const char *, char **);
void duckport_source_error_free(char *);
}

namespace duckdb {
void RegisterFileAndBlobFixtures(ExtensionLoader &loader) {
	ScalarFunction security(
	    "duckport_fixture_security", {LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [](DataChunk &args, ExpressionState &, Vector &result) {
		    UnaryExecutor::Execute<string_t, string_t>(args.data[0], result, args.size(), [&result](string_t input) {
			    const auto test_case = input.GetString();
			    DuckportSecurityOptions options {};
			    DuckportHttpHeader headers[] = {{"X-Key", "fixture-key"}, {"x-key", "fixture-second"}};
			    if (test_case == "http" || test_case == "mixed") {
				    options.token = "fixture-token";
			    }
			    if (test_case == "http") {
				    options.http_headers = headers;
				    options.http_header_count = 1;
			    }
			    if (test_case == "s3" || test_case == "s3-session" || test_case == "mixed" ||
			        test_case == "s3-anonymous-key") {
				    options.access_key_id = "fixture-access";
				    options.secret_access_key = "fixture-secret";
			    }
			    if (test_case == "s3-session" || test_case == "session-only") {
				    options.session_token = "fixture-session";
			    }
			    if (test_case == "partial") {
				    options.access_key_id = "fixture-access";
			    }
			    if (test_case == "empty-key") {
				    options.access_key_id = "";
				    options.secret_access_key = "fixture-secret";
			    }
			    if (test_case == "s3-anonymous" || test_case == "s3-anonymous-key") {
				    options.allow_anonymous = 1;
			    }
			    if (test_case == "duplicates") {
				    options.http_headers = headers;
				    options.http_header_count = 2;
			    }
			    if (test_case == "transport-header") {
				    headers[0].name = "Host";
				    options.http_headers = headers;
				    options.http_header_count = 1;
			    }
			    if (test_case == "bad-value") {
				    headers[0].value = "fixture-key\nsecond-line";
				    options.http_headers = headers;
				    options.http_header_count = 1;
			    }
			    char *raw = nullptr;
			    unique_ptr<char, decltype(&duckport_source_error_free)> error(duckport_fixture_security(&options, &raw),
			                                                                  duckport_source_error_free);
			    if (error) {
				    throw IOException("Credential fixture: %s", error.get());
			    }
			    unique_ptr<char, decltype(&duckport_source_error_free)> value(raw, duckport_source_error_free);
			    return StringVector::AddString(result, value.get());
		    });
	    });
	security.SetVolatile();
	loader.RegisterFunction(security);

	ScalarFunction layout(
	    "duckport_fixture_blob_layout", {LogicalType::VARCHAR, LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [](DataChunk &args, ExpressionState &, Vector &result) {
		    BinaryExecutor::Execute<string_t, string_t, string_t>(
		        args.data[0], args.data[1], result, args.size(), [&result](string_t path, string_t field) {
			        char *raw = nullptr;
			        unique_ptr<char, decltype(&duckport_source_error_free)> error(
			            duckport_fixture_blob_layout(path.GetString().c_str(), field.GetString().c_str(), &raw),
			            duckport_source_error_free);
			        if (error) {
				        throw IOException("Blob layout fixture: %s", error.get());
			        }
			        unique_ptr<char, decltype(&duckport_source_error_free)> value(raw, duckport_source_error_free);
			        return StringVector::AddString(result, value.get());
		        });
	    });
	layout.SetVolatile();
	loader.RegisterFunction(layout);
	ScalarFunction metadata(
	    "duckport_fixture_field_metadata", {LogicalType::VARCHAR, LogicalType::VARCHAR, LogicalType::VARCHAR},
	    LogicalType::VARCHAR, [](DataChunk &args, ExpressionState &, Vector &result) {
		    TernaryExecutor::Execute<string_t, string_t, string_t, string_t>(
		        args.data[0], args.data[1], args.data[2], result,
		        [&result](string_t path, string_t field, string_t key) {
			        char *raw = nullptr;
			        unique_ptr<char, decltype(&duckport_source_error_free)> error(
			            duckport_fixture_field_metadata(path.GetString().c_str(), field.GetString().c_str(),
			                                            key.GetString().c_str(), &raw),
			            duckport_source_error_free);
			        if (error) {
				        throw IOException("Field metadata fixture: %s", error.get());
			        }
			        unique_ptr<char, decltype(&duckport_source_error_free)> value(raw, duckport_source_error_free);
			        return StringVector::AddString(result, value.get());
		        });
	    });
	metadata.SetVolatile();
	loader.RegisterFunction(metadata);
	ScalarFunction write("duckport_fixture_write", {LogicalType::VARCHAR, LogicalType::BLOB}, LogicalType::VARCHAR,
	                     [](DataChunk &args, ExpressionState &, Vector &result) {
		                     BinaryExecutor::Execute<string_t, string_t, string_t>(
		                         args.data[0], args.data[1], result, args.size(),
		                         [&result](string_t path, string_t bytes) {
			                         auto file_path = std::filesystem::path(path.GetString());
			                         std::filesystem::create_directories(file_path.parent_path());
			                         std::ofstream file(file_path, std::ios::binary | std::ios::trunc);
			                         file.write(bytes.GetData(), bytes.GetSize());
			                         file.close();
			                         if (!file) {
				                         throw IOException("Cannot write fixture file");
			                         }
			                         return StringVector::AddString(result, path.GetString());
		                         });
	                     });
	write.SetVolatile();
	loader.RegisterFunction(write);
	ScalarFunction remove("duckport_fixture_remove", {LogicalType::VARCHAR}, LogicalType::BOOLEAN,
	                      [](DataChunk &args, ExpressionState &, Vector &result) {
		                      UnaryExecutor::Execute<string_t, bool>(
		                          args.data[0], result, args.size(), [](string_t path) {
			                          if (!std::filesystem::remove(path.GetString())) {
				                          throw IOException("Fixture file was absent");
			                          }
			                          return true;
		                          });
	                      });
	remove.SetVolatile();
	loader.RegisterFunction(remove);
}
} // namespace duckdb
