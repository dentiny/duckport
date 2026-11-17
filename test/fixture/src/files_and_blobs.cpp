#include "duckdb.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/common/vector_operations/unary_executor.hpp"
#include "duckdb/common/vector_operations/ternary_executor.hpp"
#include "duckdb/main/extension/extension_loader.hpp"
#include <filesystem>
#include <fstream>

extern "C" {
char *duckport_fixture_blob_layout(const char *, const char *, char **);
char *duckport_fixture_field_metadata(const char *, const char *, const char *, char **);
void duckport_source_error_free(char *);
}

namespace duckdb {
void RegisterFileAndBlobFixtures(ExtensionLoader &loader) {
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
