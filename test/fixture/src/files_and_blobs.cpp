#include "fixture_support.hpp"
#include "duckdb.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/common/vector_operations/unary_executor.hpp"
#include "duckdb/common/vector_operations/ternary_executor.hpp"
#include <filesystem>
#include "duckdb/common/fstream.hpp"

extern "C" {
char *duckport_fixture_blob_layout(const char *, const char *, char **);
char *duckport_fixture_field_metadata(const char *, const char *, const char *, char **);
void duckport_source_error_free(char *);
}

namespace duckdb {
namespace {
template <class Read, class... Args>
string_t ReadDiagnostic(Vector &result, const char *label, Read read, Args... args) {
	char *raw = nullptr;
	unique_ptr<char, decltype(&duckport_source_error_free)> error(read(args..., &raw), duckport_source_error_free);
	unique_ptr<char, decltype(&duckport_source_error_free)> value(raw, duckport_source_error_free);
	if (error) {
		throw IOException("%s fixture: %s", label, error.get());
	}
	return StringVector::AddString(result, value.get());
}

void BlobLayout(DataChunk &args, ExpressionState &, Vector &result) {
	BinaryExecutor::Execute<string_t, string_t, string_t>(
	    args.data[0], args.data[1], result, args.size(), [&result](string_t path, string_t field) {
		    return ReadDiagnostic(result, "Blob layout", duckport_fixture_blob_layout, path.GetString().c_str(),
		                          field.GetString().c_str());
	    });
}

string WriteFixtureFile(const string &path, string_t bytes) {
	auto file_path = std::filesystem::path(path);
	std::filesystem::create_directories(file_path.parent_path());
	ofstream file(file_path, ios::binary | ios::trunc);
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
			        return ReadDiagnostic(result, "Field metadata", duckport_fixture_field_metadata,
			                              path.GetString().c_str(), field.GetString().c_str(), key.GetString().c_str());
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
	ScalarFunction layout("duckport_fixture_blob_layout", {LogicalType::VARCHAR, LogicalType::VARCHAR},
	                      LogicalType::VARCHAR, BlobLayout);
	layout.SetVolatile();
	loader.RegisterFunction(layout);
	RegisterMetadataFixture(loader);
	RegisterWriteFixture(loader);
	RegisterRemoveFixture(loader);
}
} // namespace duckdb
