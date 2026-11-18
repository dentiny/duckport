#include "fixture_support.hpp"
#include "duckdb.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/common/vector_operations/unary_executor.hpp"
#include "duckdb/common/vector_operations/ternary_executor.hpp"
#include <filesystem>
#include <fstream>

extern "C" {
char *duckport_fixture_blob_layout(const char *, const char *, char **);
char *duckport_fixture_field_metadata(const char *, const char *, const char *, char **);
void duckport_source_error_free(char *);
}

namespace duckdb {
namespace {
string ReadDiagnostic(const char *label, const std::function<char *(char **)> &read) {
	char *raw = nullptr;
	unique_ptr<char, decltype(&duckport_source_error_free)> error(read(&raw), duckport_source_error_free);
	unique_ptr<char, decltype(&duckport_source_error_free)> value(raw, duckport_source_error_free);
	if (error) {
		throw IOException("%s fixture: %s", label, error.get());
	}
	return string(value.get());
}

string BlobLayout(const string &path, const string &field) {
	return ReadDiagnostic("Blob layout", [&](char **output) {
		return duckport_fixture_blob_layout(path.c_str(), field.c_str(), output);
	});
}

string FieldMetadata(const string &path, const string &field, const string &key) {
	return ReadDiagnostic("Field metadata", [&](char **output) {
		return duckport_fixture_field_metadata(path.c_str(), field.c_str(), key.c_str(), output);
	});
}

string WriteFixtureFile(const string &path, string_t bytes) {
	auto file_path = std::filesystem::path(path);
	std::filesystem::create_directories(file_path.parent_path());
	std::ofstream file(file_path, std::ios::binary | std::ios::trunc);
	file.write(bytes.GetData(), bytes.GetSize());
	file.close();
	if (!file) {
		throw IOException("Cannot write fixture file");
	}
	return path;
}

void RegisterMetadataFixture(ExtensionLoader &loader) {
	ScalarFunction metadata(
	    "duckport_fixture_field_metadata", {LogicalType::VARCHAR, LogicalType::VARCHAR, LogicalType::VARCHAR},
	    LogicalType::VARCHAR, [](DataChunk &args, ExpressionState &, Vector &result) {
		    TernaryExecutor::Execute<string_t, string_t, string_t, string_t>(
		        args.data[0], args.data[1], args.data[2], result,
		        [&result](string_t path, string_t field, string_t key) {
			        return StringVector::AddString(result,
			                                       FieldMetadata(path.GetString(), field.GetString(), key.GetString()));
		        });
	    });
	metadata.SetVolatile();
	loader.RegisterFunction(metadata);
}

void RegisterWriteFixture(ExtensionLoader &loader) {
	ScalarFunction write("duckport_fixture_write", {LogicalType::VARCHAR, LogicalType::BLOB}, LogicalType::VARCHAR,
	                     [](DataChunk &args, ExpressionState &, Vector &result) {
		                     BinaryExecutor::Execute<string_t, string_t, string_t>(
		                         args.data[0], args.data[1], result, args.size(),
		                         [&result](string_t path, string_t bytes) {
			                         return StringVector::AddString(result, WriteFixtureFile(path.GetString(), bytes));
		                         });
	                     });
	write.SetVolatile();
	loader.RegisterFunction(write);
}

void RegisterRemoveFixture(ExtensionLoader &loader) {
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
} // namespace

void RegisterFileAndBlobFixtures(ExtensionLoader &loader) {
	RegisterBinaryFixtureCommand(loader, "duckport_fixture_blob_layout", BlobLayout);
	RegisterMetadataFixture(loader);
	RegisterWriteFixture(loader);
	RegisterRemoveFixture(loader);
}
} // namespace duckdb
