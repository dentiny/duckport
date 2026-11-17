#include "iceberg.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "httplib.hpp"
#include "yyjson.hpp"

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <memory>
#include <mutex>
#include <thread>
#include <unordered_map>

namespace duckdb {
namespace {
using namespace duckdb_yyjson;
using Document = std::unique_ptr<yyjson_mut_doc, decltype(&yyjson_mut_doc_free)>;

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

string Text(yyjson_mut_val *value) {
	auto text = yyjson_mut_get_str(value);
	if (!text) {
		throw InvalidInputException("Expected a fixture JSON string");
	}
	return text;
}

class IcebergFixture {
public:
	explicit IcebergFixture(const string &root) : metadata(nullptr, yyjson_mut_doc_free) {
		std::ifstream seed(string(DUCKPORT_TESTDATA_DIR) + "/iceberg/table.metadata.json");
		if (!seed) {
			throw IOException("Cannot open Iceberg metadata fixture");
		}
		metadata = Parse(string(std::istreambuf_iterator<char>(seed), {}));
		auto table_path = std::filesystem::absolute(std::filesystem::path(root) / "people");
		directory = table_path / "metadata";
		std::filesystem::create_directories(directory);
		Put(metadata.get(), yyjson_mut_doc_get_root(metadata.get()), "location",
		    yyjson_mut_strcpy(metadata.get(), table_path.generic_string().c_str()));
		metadata_file = Publish(metadata.get(), 0);
		server.Get("/v1/config", [](const duckdb_httplib::Request &, duckdb_httplib::Response &res) {
			res.set_content(R"({"defaults":{},"overrides":{}})", "application/json");
		});
		server.Get(TABLE_ROUTE, [this](const duckdb_httplib::Request &, duckdb_httplib::Response &res) {
			std::lock_guard<std::mutex> guard(mutex);
			Reply(res);
		});
		server.Post(TABLE_ROUTE, [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			std::lock_guard<std::mutex> guard(mutex);
			try {
				Commit(req.body);
				Reply(res);
			} catch (const std::exception &) {
				res.status = 409;
				res.set_content(
				    R"({"error":{"message":"fixture commit rejected","type":"CommitFailedException","code":409}})",
				    "application/json");
			}
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

	string Command(const string &command) {
		std::lock_guard<std::mutex> guard(mutex);
		if (command == "start") {
			return endpoint;
		}
		if (command == "metadata") {
			return metadata_file;
		}
		if (command == "commits") {
			return std::to_string(commits);
		}
		if (command == "reject_commits") {
			reject_commits = true;
			return "true";
		}
		throw InvalidInputException("Unknown Iceberg fixture command: %s", command);
	}

private:
	static constexpr const char *TABLE_ROUTE = "/v1/namespaces/tests/tables/people";

	string Publish(yyjson_mut_doc *doc, size_t version) {
		auto path = directory / (std::to_string(version) + ".metadata.json");
		std::ofstream output(path);
		output << Encode(yyjson_mut_doc_get_root(doc));
		output.close();
		if (!output) {
			throw IOException("Cannot publish Iceberg fixture metadata");
		}
		return path.generic_string();
	}

	void Reply(duckdb_httplib::Response &res) {
		Document response(yyjson_mut_doc_new(nullptr), yyjson_mut_doc_free);
		auto root = yyjson_mut_obj(response.get());
		yyjson_mut_doc_set_root(response.get(), root);
		Put(response.get(), root, "metadata-location", yyjson_mut_strcpy(response.get(), metadata_file.c_str()));
		Put(response.get(), root, "metadata",
		    yyjson_mut_val_mut_copy(response.get(), yyjson_mut_doc_get_root(metadata.get())));
		Put(response.get(), root, "config", yyjson_mut_obj(response.get()));
		res.set_content(Encode(root), "application/json");
	}

	void Commit(const string &body) {
		if (reject_commits) {
			throw InvalidInputException("Injected commit conflict");
		}
		auto request = Parse(body);
		auto request_root = yyjson_mut_doc_get_root(request.get());
		auto current = yyjson_mut_doc_get_root(metadata.get());
		auto requirements = yyjson_mut_obj_get(request_root, "requirements");
		size_t index, count;
		yyjson_mut_val *item;
		yyjson_mut_arr_foreach(requirements, index, count, item) {
			auto type = Text(yyjson_mut_obj_get(item, "type"));
			bool matches = false;
			if (type == "assert-table-uuid") {
				matches = Encode(yyjson_mut_obj_get(item, "uuid")) == Encode(yyjson_mut_obj_get(current, "table-uuid"));
			} else if (type == "assert-ref-snapshot-id") {
				auto ref = Text(yyjson_mut_obj_get(item, "ref"));
				auto existing = yyjson_mut_obj_get(yyjson_mut_obj_get(current, "refs"), ref.c_str());
				auto actual = yyjson_mut_obj_get(existing, "snapshot-id");
				auto expected = yyjson_mut_obj_get(item, "snapshot-id");
				matches =
				    actual ? expected && Encode(actual) == Encode(expected) : !expected || yyjson_mut_is_null(expected);
			}
			if (!matches) {
				throw InvalidInputException("Iceberg commit requirement failed");
			}
		}
		Document next(yyjson_mut_doc_mut_copy(metadata.get(), nullptr), yyjson_mut_doc_free);
		auto doc = next.get();
		auto root = yyjson_mut_doc_get_root(doc);
		auto updates = yyjson_mut_obj_get(request_root, "updates");
		yyjson_mut_arr_foreach(updates, index, count, item) {
			auto action = Text(yyjson_mut_obj_get(item, "action"));
			if (action == "add-snapshot") {
				auto snapshot = yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(item, "snapshot"));
				yyjson_mut_arr_append(yyjson_mut_obj_get(root, "snapshots"), snapshot);
				Put(doc, root, "last-sequence-number",
				    yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(snapshot, "sequence-number")));
				Put(doc, root, "last-updated-ms",
				    yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(snapshot, "timestamp-ms")));
			} else if (action == "set-snapshot-ref") {
				auto name = Text(yyjson_mut_obj_get(item, "ref-name"));
				auto ref = yyjson_mut_val_mut_copy(doc, item);
				yyjson_mut_obj_remove_key(ref, "action");
				yyjson_mut_obj_remove_key(ref, "ref-name");
				Put(doc, yyjson_mut_obj_get(root, "refs"), name.c_str(), ref);
				if (name == "main") {
					auto snapshot_id = yyjson_mut_obj_get(ref, "snapshot-id");
					Put(doc, root, "current-snapshot-id", yyjson_mut_val_mut_copy(doc, snapshot_id));
					auto log = yyjson_mut_obj(doc);
					Put(doc, log, "snapshot-id", yyjson_mut_val_mut_copy(doc, snapshot_id));
					Put(doc, log, "timestamp-ms",
					    yyjson_mut_val_mut_copy(doc, yyjson_mut_obj_get(root, "last-updated-ms")));
					yyjson_mut_arr_append(yyjson_mut_obj_get(root, "snapshot-log"), log);
				}
			} else {
				throw InvalidInputException("Unsupported Iceberg fixture update: %s", action);
			}
		}
		auto file = Publish(doc, commits + 1);
		metadata = std::move(next);
		metadata_file = std::move(file);
		commits++;
	}

	duckdb_httplib::Server server;
	std::thread worker;
	std::mutex mutex;
	Document metadata;
	std::filesystem::path directory;
	string endpoint;
	string metadata_file;
	size_t commits = 0;
	bool reject_commits = false;
};

string FixtureCommand(const string &root, const string &command) {
	static std::mutex mutex;
	static std::unordered_map<string, std::unique_ptr<IcebergFixture>> fixtures;
	std::lock_guard<std::mutex> guard(mutex);
	if (command == "stop") {
		fixtures.erase(root);
		return "stopped";
	}
	auto entry = fixtures.find(root);
	if (entry == fixtures.end()) {
		if (command != "start") {
			throw InvalidInputException("Iceberg fixture has not been started");
		}
		entry = fixtures.emplace(root, std::make_unique<IcebergFixture>(root)).first;
	}
	return entry->second->Command(command);
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
