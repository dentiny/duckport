#include "iceberg.hpp"
#include "fixture_support.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/helper.hpp"
#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/common/unordered_map.hpp"
#include "duckdb/common/mutex.hpp"
#include "httplib.hpp"
#include "yyjson.hpp"

#include <cstdlib>
#include <filesystem>
#include <fstream>

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

// One fixed-schema table; the catalog fixture serializes access under its mutex.
class IcebergTableFixture {
public:
	explicit IcebergTableFixture(const string &root, bool empty) : metadata(nullptr, yyjson_mut_doc_free) {
		metadata = Parse(ReadFixtureFile("iceberg", "table.metadata.json"));
		auto table_path = std::filesystem::absolute(std::filesystem::path(root) / "people");
		std::filesystem::create_directories(table_path / "metadata");
		metadata_file = (table_path / "metadata/current.metadata.json").generic_string();
		Put(metadata.get(), yyjson_mut_doc_get_root(metadata.get()), "location",
		    yyjson_mut_strcpy(metadata.get(), table_path.generic_string().c_str()));
		auto response = yyjson_mut_obj(metadata.get());
		Put(metadata.get(), response, "metadata", yyjson_mut_doc_get_root(metadata.get()));
		Put(metadata.get(), response, "metadata-location", yyjson_mut_strcpy(metadata.get(), metadata_file.c_str()));
		yyjson_mut_doc_set_root(metadata.get(), response);
		exists = !empty;
		if (exists) {
			Publish(metadata.get());
		}
	}

	bool Exists() const {
		return exists;
	}

	string Response() const {
		return Encode(yyjson_mut_doc_get_root(metadata.get()));
	}

	// Creation tests use the same fixed people schema as append tests.
	bool Create(const string &body) {
		auto request = Parse(body);
		auto schema = yyjson_mut_obj_get(yyjson_mut_doc_get_root(request.get()), "schema");
		auto table = yyjson_mut_obj_get(yyjson_mut_doc_get_root(metadata.get()), "metadata");
		auto expected = yyjson_mut_arr_get(yyjson_mut_obj_get(table, "schemas"), 0);
		if (!yyjson_mut_equals(yyjson_mut_obj_get(schema, "fields"), yyjson_mut_obj_get(expected, "fields"))) {
			return false;
		}
		Publish(metadata.get());
		exists = true;
		return true;
	}

	// Publish only after every update has been validated and applied to a copy.
	void Commit(const string &body) {
		auto request = Parse(body);
		Document next(yyjson_mut_doc_mut_copy(metadata.get(), nullptr), yyjson_mut_doc_free);
		ApplyUpdates(next.get(), yyjson_mut_doc_get_root(request.get()));
		Publish(next.get());
		metadata = std::move(next);
	}

private:
	void Publish(yyjson_mut_doc *doc) {
		std::ofstream output(metadata_file);
		output << Encode(yyjson_mut_obj_get(yyjson_mut_doc_get_root(doc), "metadata"));
		output.close();
		if (!output) {
			throw IOException("Cannot publish Iceberg fixture metadata");
		}
	}

	static void ApplyUpdates(yyjson_mut_doc *doc, yyjson_mut_val *request) {
		auto updates = yyjson_mut_obj_get(request, "updates");
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
		ApplySnapshot(doc, snapshot, ref);
	}

	static void ApplySnapshot(yyjson_mut_doc *doc, yyjson_mut_val *snapshot, yyjson_mut_val *ref) {
		auto root = yyjson_mut_obj_get(yyjson_mut_doc_get_root(doc), "metadata");
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
	}

	Document metadata;
	string metadata_file;
	bool exists;
};

class IcebergCatalogFixture {
public:
	explicit IcebergCatalogFixture(const string &root, bool authenticated = false, bool empty = false)
	    : table(root, empty) {
		ConfigureAuthentication(authenticated);
		RegisterRoutes();
		server.Start();
	}

	~IcebergCatalogFixture() {
		server.Stop();
	}

	const string &Endpoint() const {
		return server.Endpoint();
	}

private:
	static constexpr const char *TABLE_ROUTE = "/v1/namespaces/tests/tables/people";

	void ConfigureAuthentication(bool authenticated) {
		server.Http().set_pre_routing_handler(
		    [authenticated](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			    if (authenticated && req.get_header_value("Authorization") != "Bearer fixture-catalog-key") {
				    res.status = 401;
				    res.set_content("catalog token required", "text/plain");
				    return duckdb_httplib::Server::HandlerResponse::Handled;
			    }
			    return duckdb_httplib::Server::HandlerResponse::Unhandled;
		    });
	}

	void RegisterRoutes() {
		server.Http().Get("/v1/config", [](const duckdb_httplib::Request &, duckdb_httplib::Response &res) {
			res.set_content(R"({"defaults":{},"overrides":{}})", "application/json");
		});
		server.Http().Get(TABLE_ROUTE,
		                  [this](const duckdb_httplib::Request &, duckdb_httplib::Response &res) { LoadTable(res); });
		server.Http().Post(
		    "/v1/namespaces/tests/tables",
		    [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) { CreateTable(req.body, res); });
		server.Http().Post(TABLE_ROUTE, [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			CommitTable(req.body, res);
		});
	}

	void LoadTable(duckdb_httplib::Response &res) {
		annotated_lock_guard<annotated_mutex> guard(mutex);
		if (!table.Exists()) {
			res.status = 404;
			res.set_content(R"({"error":{"message":"Table not found","type":"NoSuchTableException","code":404}})",
			                "application/json");
			return;
		}
		res.set_content(table.Response(), "application/json");
	}

	void CreateTable(const string &body, duckdb_httplib::Response &res) {
		annotated_lock_guard<annotated_mutex> guard(mutex);
		if (table.Exists()) {
			res.status = 409;
			res.set_content(
			    R"({"error":{"message":"Table already exists","type":"AlreadyExistsException","code":409}})",
			    "application/json");
			return;
		}
		if (!table.Create(body)) {
			res.status = 400;
			return;
		}
		res.set_content(table.Response(), "application/json");
	}

	void CommitTable(const string &body, duckdb_httplib::Response &res) {
		annotated_lock_guard<annotated_mutex> guard(mutex);
		table.Commit(body);
		res.set_content(table.Response(), "application/json");
	}

	FixtureServer server {"Iceberg catalog fixture"};
	annotated_mutex mutex;
	IcebergTableFixture table DUCKDB_GUARDED_BY(mutex);
};

string FixtureCommand(const string &root, const string &command) {
	static annotated_mutex mutex;
	static unordered_map<string, unique_ptr<IcebergCatalogFixture>> fixtures DUCKDB_GUARDED_BY(mutex);
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
		fixture = make_uniq<IcebergCatalogFixture>(root, command == "start_authenticated", command == "start_empty");
	}
	return fixture->Endpoint();
}
} // namespace

void RegisterIcebergFixture(ExtensionLoader &loader) {
	RegisterBinaryFixtureCommand(loader, "duckport_iceberg_fixture", FixtureCommand);
}
} // namespace duckdb
