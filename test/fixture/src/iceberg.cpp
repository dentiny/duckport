#include "iceberg.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/helper.hpp"
#include "duckdb/common/mutex.hpp"
#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/common/unordered_map.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "httplib.hpp"
#include "yyjson.hpp"

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <thread>

namespace duckdb {
namespace {
using namespace duckdb_yyjson;
using Document = unique_ptr<yyjson_mut_doc, decltype(&yyjson_mut_doc_free)>;

Document Parse(const string &text) {
	auto input = yyjson_read(text.data(), text.size(), 0);
	if (!input) {
		throw InvalidInputException("Invalid fixture JSON");
	}
	Document result(yyjson_doc_mut_copy(input, nullptr), yyjson_mut_doc_free);
	yyjson_doc_free(input);
	return result;
}

string Encode(yyjson_mut_val *value) {
	size_t size;
	auto buffer = yyjson_mut_val_write(value, 0, &size);
	if (!buffer) {
		throw IOException("Cannot serialize fixture JSON");
	}
	string result(buffer, size);
	free(buffer);
	return result;
}

void Put(yyjson_mut_doc *doc, yyjson_mut_val *object, const char *key, yyjson_mut_val *value) {
	yyjson_mut_obj_put(object, yyjson_mut_strcpy(doc, key), value);
}

// Include nested struct, list, and map field IDs in table metadata.
int64_t MaxFieldId(yyjson_mut_val *value) {
	int64_t result = 0;
	size_t index, count;
	yyjson_mut_val *item;
	if (yyjson_mut_is_arr(value)) {
		yyjson_mut_arr_foreach(value, index, count, item) {
			result = MaxValue(result, MaxFieldId(item));
		}
	} else if (yyjson_mut_is_obj(value)) {
		yyjson_mut_val *key;
		yyjson_mut_obj_foreach(value, index, count, key, item) {
			if (yyjson_mut_equals_str(key, "id") || yyjson_mut_equals_str(key, "element-id") ||
			    yyjson_mut_equals_str(key, "key-id") || yyjson_mut_equals_str(key, "value-id")) {
				result = MaxValue(result, yyjson_mut_get_sint(item));
			} else {
				result = MaxValue(result, MaxFieldId(item));
			}
		}
	}
	return result;
}

class IcebergFixture {
public:
	explicit IcebergFixture(const string &root, bool authenticated = false, bool empty = false)
	    : metadata(nullptr, yyjson_mut_doc_free) {
		std::ifstream seed(string(DUCKPORT_TESTDATA_DIR) + "/iceberg/table.metadata.json");
		if (!seed) {
			throw IOException("Cannot open Iceberg metadata fixture");
		}
		metadata = Parse(string(std::istreambuf_iterator<char>(seed), {}));
		auto table_path = std::filesystem::absolute(std::filesystem::path(root) / "people");
		std::filesystem::create_directories(table_path / "metadata");
		metadata_file = (table_path / "metadata/current.metadata.json").generic_string();
		Put(metadata.get(), yyjson_mut_doc_get_root(metadata.get()), "location",
		    yyjson_mut_strcpy(metadata.get(), table_path.generic_string().c_str()));
		auto response = yyjson_mut_obj(metadata.get());
		Put(metadata.get(), response, "metadata", yyjson_mut_doc_get_root(metadata.get()));
		Put(metadata.get(), response, "metadata-location", yyjson_mut_strcpy(metadata.get(), metadata_file.c_str()));
		yyjson_mut_doc_set_root(metadata.get(), response);
		if (!empty) {
			Publish(metadata.get());
		}
		exists = !empty;
		server.set_pre_routing_handler(
		    [authenticated](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			    if (authenticated && req.get_header_value("Authorization") != "Bearer fixture-catalog-key") {
				    res.status = 401;
				    res.set_content("catalog token required", "text/plain");
				    return duckdb_httplib::Server::HandlerResponse::Handled;
			    }
			    return duckdb_httplib::Server::HandlerResponse::Unhandled;
		    });
		server.Get("/v1/config", [](const duckdb_httplib::Request &, duckdb_httplib::Response &res) {
			res.set_content(R"({"defaults":{},"overrides":{}})", "application/json");
		});
		server.Get(TABLE_ROUTE, [this](const duckdb_httplib::Request &, duckdb_httplib::Response &res) {
			annotated_lock_guard<annotated_mutex> guard(mutex);
			if (!exists) {
				res.status = 404;
				res.set_content(R"({"error":{"message":"Table not found","type":"NoSuchTableException","code":404}})",
				                "application/json");
				return;
			}
			res.set_content(Encode(yyjson_mut_doc_get_root(metadata.get())), "application/json");
		});
		server.Post(
		    "/v1/namespaces/tests/tables", [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			    annotated_lock_guard<annotated_mutex> guard(mutex);
			    if (exists) {
				    res.status = 409;
				    res.set_content(
				        R"({"error":{"message":"Table already exists","type":"AlreadyExistsException","code":409}})",
				        "application/json");
				    return;
			    }
			    auto request = Parse(req.body);
			    auto input = yyjson_mut_doc_get_root(request.get());
			    if (!yyjson_mut_equals_str(yyjson_mut_obj_get(input, "name"), "people")) {
				    res.status = 400;
				    return;
			    }
			    auto doc = metadata.get();
			    auto root = yyjson_mut_obj_get(yyjson_mut_doc_get_root(doc), "metadata");
			    auto schema = yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(input, "schema"));
			    Put(doc, schema, "schema-id", yyjson_mut_int(doc, 0));
			    auto schemas = yyjson_mut_arr(doc);
			    yyjson_mut_arr_append(schemas, schema);
			    Put(doc, root, "schemas", schemas);
			    Put(doc, root, "last-column-id", yyjson_mut_int(doc, MaxFieldId(schema)));
			    Publish(doc);
			    exists = true;
			    res.set_content(Encode(yyjson_mut_doc_get_root(doc)), "application/json");
		    });
		server.Post(TABLE_ROUTE, [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			annotated_lock_guard<annotated_mutex> guard(mutex);
			Commit(req.body);
			res.set_content(Encode(yyjson_mut_doc_get_root(metadata.get())), "application/json");
		});
		auto port = server.bind_to_any_port("127.0.0.1");
		if (port < 0) {
			throw IOException("Cannot bind Iceberg catalog fixture");
		}
		endpoint = "http://127.0.0.1:" + std::to_string(port);
		worker = std::thread([this] { server.listen_after_bind(); });
		server.wait_until_ready();
	}

	~IcebergFixture() {
		server.stop();
		if (worker.joinable()) {
			worker.join();
		}
	}

	const string &Endpoint() const {
		return endpoint;
	}

private:
	static constexpr const char *TABLE_ROUTE = "/v1/namespaces/tests/tables/people";

	void Publish(yyjson_mut_doc *doc) {
		std::ofstream output(metadata_file);
		output << Encode(yyjson_mut_obj_get(yyjson_mut_doc_get_root(doc), "metadata"));
		output.close();
		if (!output) {
			throw IOException("Cannot publish Iceberg fixture metadata");
		}
	}

	// Single-writer fixture: publish the main-branch snapshot produced by COPY.
	void Commit(const string &body) DUCKDB_REQUIRES(mutex) {
		auto request = Parse(body);
		Document next(yyjson_mut_doc_mut_copy(metadata.get(), nullptr), yyjson_mut_doc_free);
		auto doc = next.get();
		auto root = yyjson_mut_obj_get(yyjson_mut_doc_get_root(doc), "metadata");
		auto updates = yyjson_mut_obj_get(yyjson_mut_doc_get_root(request.get()), "updates");
		yyjson_mut_val *snapshot = nullptr, *ref = nullptr, *item;
		size_t index, count;
		yyjson_mut_arr_foreach(updates, index, count, item) {
			auto action = yyjson_mut_obj_get(item, "action");
			if (yyjson_mut_equals_str(action, "add-snapshot")) {
				snapshot = yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(item, "snapshot"));
			} else if (yyjson_mut_equals_str(action, "set-snapshot-ref") &&
			           yyjson_mut_equals_str(yyjson_mut_obj_get(item, "ref-name"), "main")) {
				ref = yyjson_mut_val_mut_copy(doc, item);
			} else {
				throw InvalidInputException("Unsupported Iceberg fixture update");
			}
		}
		if (!snapshot || !ref) {
			throw InvalidInputException("Expected an Iceberg snapshot and main ref");
		}
		yyjson_mut_arr_append(yyjson_mut_obj_get(root, "snapshots"), snapshot);
		Put(doc, root, "last-sequence-number",
		    yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(snapshot, "sequence-number")));
		Put(doc, root, "last-updated-ms", yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(snapshot, "timestamp-ms")));
		Put(doc, root, "current-snapshot-id",
		    yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(snapshot, "snapshot-id")));
		yyjson_mut_obj_remove_key(ref, "action");
		yyjson_mut_obj_remove_key(ref, "ref-name");
		Put(doc, yyjson_mut_obj_get(root, "refs"), "main", ref);
		auto log = yyjson_mut_obj(doc);
		Put(doc, log, "snapshot-id", yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(snapshot, "snapshot-id")));
		Put(doc, log, "timestamp-ms", yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(snapshot, "timestamp-ms")));
		yyjson_mut_arr_append(yyjson_mut_obj_get(root, "snapshot-log"), log);
		Publish(doc);
		metadata = std::move(next);
	}

	duckdb_httplib::Server server;
	std::thread worker;
	annotated_mutex mutex;
	Document metadata DUCKDB_GUARDED_BY(mutex);
	string endpoint;
	string metadata_file;
	bool exists DUCKDB_GUARDED_BY(mutex);
};

string FixtureCommand(const string &root, const string &command) {
	static annotated_mutex mutex;
	static unordered_map<string, unique_ptr<IcebergFixture>> fixtures DUCKDB_GUARDED_BY(mutex);
	annotated_lock_guard<annotated_mutex> guard(mutex);
	if (command == "stop") {
		fixtures.erase(root);
		return "stopped";
	}
	if (command != "start" && command != "start_authenticated" && command != "start_empty") {
		throw InvalidInputException("Unknown Iceberg fixture command: %s", command);
	}
	auto &fixture = fixtures[root];
	if (!fixture) {
		fixture = make_uniq<IcebergFixture>(root, command == "start_authenticated", command == "start_empty");
	}
	return fixture->Endpoint();
}
} // namespace

void RegisterIcebergFixture(ExtensionLoader &loader) {
	ScalarFunction command(
	    "duckport_iceberg_fixture", {LogicalType::VARCHAR, LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [](DataChunk &args, ExpressionState &, Vector &result) {
		    BinaryExecutor::Execute<string_t, string_t, string_t>(
		        args.data[0], args.data[1], result, args.size(), [&result](string_t root, string_t command) {
			        return StringVector::AddString(result, FixtureCommand(root.GetString(), command.GetString()));
		        });
	    });
	command.SetVolatile();
	loader.RegisterFunction(command);
}
} // namespace duckdb
