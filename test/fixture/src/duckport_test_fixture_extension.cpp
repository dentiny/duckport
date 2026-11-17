#define DUCKDB_EXTENSION_MAIN
#include "duckport_test_fixture_extension.hpp"
#include "iceberg.hpp"
#include "duckdb/common/helper.hpp"
#include "duckdb/common/map.hpp"
#include "duckdb/common/shared_ptr.hpp"
#include "duckdb/common/string.hpp"
#include "duckdb/common/unique_ptr.hpp"
#include "duckdb/common/vector.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/common/vector_operations/unary_executor.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/function/table_function.hpp"
#include "httplib.hpp"
#include "yyjson.hpp"

#include <atomic>
#include <cstdlib>
#include <cctype>
#include <fstream>
#include <iterator>
#include <mutex>
#include <thread>

namespace duckdb {
void RegisterFileAndBlobFixtures(ExtensionLoader &loader);
namespace {
using namespace duckdb_yyjson;

void SetFixtureEnvironment(const char *name, const char *value) {
#ifdef _WIN32
    _putenv_s(name, value ? value : "");
#else
    if (value) { setenv(name, value, 1); } else { unsetenv(name); }
#endif
}

string ReadFile(const string &provider, const string &name) {
	std::ifstream file(string(DUCKPORT_TESTDATA_DIR) + "/" + provider + "/" + name, std::ios::binary);
	if (!file) {
		throw IOException("Cannot open %s fixture: %s", provider, name);
	}
	return string(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
}

void ReplaceAll(string &value, const string &from, const string &to) {
	size_t position = 0;
	while ((position = value.find(from, position)) != string::npos) {
		value.replace(position, from.size(), to);
		position += to.size();
	}
}

string JsonString(yyjson_val *object, const char *key, const string &fallback = "") {
	auto value = yyjson_obj_get(object, key);
	if (!value) {
		return fallback;
	}
	if (!yyjson_is_str(value)) {
		throw InvalidInputException("Fixture property must be a string: %s", key);
	}
	return string(yyjson_get_str(value), yyjson_get_len(value));
}

struct Route {
	string path;
	string file;
	string body;
	string redirect;
	int status = 200;
	bool is_download = false;
    string auth_header;
    string auth_value;
    bool auth_prefix = false;
    string session_token;
};

struct Trace {
	string origin;
	string target;
	string authorization;
	string api_key;
	string headers;
	string request_payer;
	bool is_download;
	bool has_authorization;
	bool has_request_payer;
};

// Only transport, committed response data, and request evidence live here.
// Expected values and assertions belong in SQLLogicTest files.
class ProviderFixture {
public:
	ProviderFixture(string provider_p, bool authenticated) : provider(std::move(provider_p)) {
		try {
			auto api_port = api.bind_to_any_port("127.0.0.1");
			auto download_port = downloads.bind_to_any_port("127.0.0.1");
			if (api_port < 0 || download_port < 0) {
				throw IOException("Cannot bind provider fixture servers");
			}
			endpoint = "http://127.0.0.1:" + std::to_string(api_port);
			download_endpoint = "http://127.0.0.1:" + std::to_string(download_port);
			LoadRoutes();
			require_bearer = authenticated && auth == "authorization";
			require_low = authenticated && provider == "internet_archive";
            api.Post(".*", [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
                Respond(req, res, api_routes, "api");
            });
			api.Get(".*", [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
				Respond(req, res, api_routes, "api");
			});
			downloads.Get(".*", [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
				Respond(req, res, download_routes, "download");
			});
			api_worker = std::thread([this] { api.listen_after_bind(); });
			download_worker = std::thread([this] { downloads.listen_after_bind(); });
			api.wait_until_ready();
			downloads.wait_until_ready();
            if (provider == "blob_hf") {
                // This dedicated SQL fixture temporarily redirects HF's standard
                // test endpoint; restore the process environment on stop.
                if (auto old = std::getenv("HF_ENDPOINT")) { old_hf_endpoint = old; had_hf_endpoint = true; }
                if (auto old = std::getenv("HF_HUB_DISABLE_IMPLICIT_TOKEN")) { old_hf_implicit = old; had_hf_implicit = true; }
                SetFixtureEnvironment("HF_ENDPOINT", endpoint.c_str());
                SetFixtureEnvironment("HF_HUB_DISABLE_IMPLICIT_TOKEN", "1");
                hf_environment = true;
            }
			if (!api.is_running() || !downloads.is_running()) {
				throw IOException("Cannot start provider fixture servers");
			}
		} catch (...) {
			Shutdown();
			throw;
		}
	}

	~ProviderFixture() {
		Shutdown();
	}

	vector<Trace> Snapshot() {
		std::lock_guard<std::mutex> guard(trace_mutex);
		return traces;
	}

	void Reset() {
		std::lock_guard<std::mutex> guard(trace_mutex);
		traces.clear();
	}

	string endpoint;
	string download_endpoint;
	string auth_scope = "/";
	std::atomic<bool> reject_metadata {false};

private:
	void Shutdown() {
		api.stop();
		downloads.stop();
		if (api_worker.joinable()) {
			api_worker.join();
		}
		if (download_worker.joinable()) {
			download_worker.join();
		}
        if (hf_environment) {
            if (had_hf_endpoint) { SetFixtureEnvironment("HF_ENDPOINT", old_hf_endpoint.c_str()); } else { SetFixtureEnvironment("HF_ENDPOINT", nullptr); }
            if (had_hf_implicit) { SetFixtureEnvironment("HF_HUB_DISABLE_IMPLICIT_TOKEN", old_hf_implicit.c_str()); } else { SetFixtureEnvironment("HF_HUB_DISABLE_IMPLICIT_TOKEN", nullptr); }
            hf_environment = false;
        }
	}

	string Expand(string value) {
		ReplaceAll(value, "API_ORIGIN", endpoint);
		ReplaceAll(value, "ORIGIN", download_endpoint);
		return value;
	}

	void LoadCollection(yyjson_val *root, const char *key, bool foreign, vector<Route> &routes) {
		auto array = yyjson_obj_get(root, key);
		if (!array) {
			return;
		}
		if (!yyjson_is_arr(array)) {
			throw InvalidInputException("Fixture property must be an array: %s", key);
		}
		for (size_t index = 0; index < yyjson_arr_size(array); index++) {
			auto entry = yyjson_arr_get(array, index);
			Route route;
			route.path = JsonString(entry, "path");
			route.file = JsonString(entry, "file");
			if (route.path.empty() || route.file.empty()) {
				throw InvalidInputException("Fixture route requires path and file");
			}
			route.body = Expand(ReadFile(provider, route.file));
			route.redirect = Expand(JsonString(entry, "redirect"));
            route.auth_header = JsonString(entry, "auth_header");
            route.auth_value = JsonString(entry, "auth_value");
            route.auth_prefix = JsonString(entry, "auth_match") == "prefix";
            route.session_token = JsonString(entry, "session_token");
			auto status = yyjson_obj_get(entry, "status");
			if (status && !yyjson_is_int(status)) {
				throw InvalidInputException("Fixture route status must be an integer");
			}
			route.status = status ? int(yyjson_get_int(status)) : (route.redirect.empty() ? 200 : 302);
			route.is_download = foreign || route.file == "attachment.bin" || route.file.rfind("records.", 0) == 0 ||
			                    (route.file.size() >= 4 && route.file.substr(route.file.size() - 4) == ".tar");
			routes.push_back(std::move(route));
		}
	}

	void LoadRoutes() {
		if (provider == "internet_archive") {
			api_routes.push_back(
			    {"/metadata/sample-item", "metadata.json", ReadFile(provider, "metadata.json"), "", 200, false});
			api_routes.push_back({"/download/sample-item/papers/paper #1%.pdf", "attachment.bin",
			                      ReadFile(provider, "attachment.bin"), "", 200, true});
			reject_route = "/metadata/sample-item";
			return;
		}
		if (provider == "arxiv") {
			for (const auto &name : {"arXiv_src_manifest.xml", "arXiv_src_2410_001.tar"}) {
				string file = string("src/") + name;
				auto body = ReadFile(provider, file);
				bool payload = file.size() >= 4 && file.substr(file.size() - 4) == ".tar";
				api_routes.push_back({"/arxiv-test/" + file, file, body, "", 200, payload});
				api_routes.push_back({"/" + file, file, body, "", 200, payload});
			}
			return;
		}
		auto json = ReadFile(provider, "http.json");
		duckdb::unique_ptr<yyjson_doc, decltype(&yyjson_doc_free)> document(yyjson_read(json.data(), json.size(), 0),
		                                                                    yyjson_doc_free);
		if (!document) {
			throw InvalidInputException("Cannot parse provider fixture http.json: %s", provider);
		}
		auto root = yyjson_doc_get_root(document.get());
		if (!yyjson_is_obj(root)) {
			throw InvalidInputException("Provider fixture http.json must be an object");
		}
		auth = JsonString(root, "auth");
		auth_scope = JsonString(root, "auth_scope", "/");
		reject_route = JsonString(root, "reject_route");
		LoadCollection(root, "routes", false, api_routes);
		LoadCollection(root, "downloads", true, download_routes);
	}

	static const Route *FindRoute(const vector<Route> &routes, const duckdb_httplib::Request &req) {
		for (const auto &key : {req.target, req.target.substr(0, req.target.find('?')), req.path}) {
			for (const auto &route : routes) {
				if (route.path == key) {
					return &route;
				}
			}
		}
		return nullptr;
	}

	void Respond(const duckdb_httplib::Request &req, duckdb_httplib::Response &res, const vector<Route> &routes,
	             const string &origin) {
		auto route = FindRoute(routes, req);
		string headers;
		for (const auto &header : req.headers) {
			headers += header.first + ": " + header.second + "\n";
		}
		Trace trace {origin,
		             req.target,
		             req.get_header_value("Authorization"),
		             req.get_header_value("x-api-key"),
		             headers,
		             req.get_header_value("x-amz-request-payer"),
		             origin == "download" || (route && route->is_download),
		             req.has_header("Authorization"),
		             req.has_header("x-amz-request-payer")};
		{
			std::lock_guard<std::mutex> guard(trace_mutex);
			traces.push_back(std::move(trace));
		}
		const auto expected_token = provider == "figshare" ? "token fixture-key" : "Bearer fixture-key";
		if (origin == "api" && require_bearer && req.get_header_value("Authorization") != expected_token) {
			res.status = 401;
			res.set_content("authentication required", "text/plain");
			return;
		}
		if (origin == "api" && require_low &&
		    req.get_header_value("Authorization") != "LOW fixture-key:fixture-secret") {
			res.status = 401;
			res.set_content("Internet Archive credentials required", "text/plain");
			return;
		}
        if (route && !route->auth_header.empty() && (route->auth_prefix ? req.get_header_value(route->auth_header).rfind(route->auth_value, 0) != 0 : req.get_header_value(route->auth_header) != route->auth_value)) {
            res.status = 401;
            res.set_content("source credentials required", "text/plain");
            return;
        }
        if (route && !route->session_token.empty() && req.get_header_value("x-amz-security-token") != route->session_token) {
            res.status = 403; res.set_content("source session token required", "text/plain"); return;
        }
		if (!route) {
			res.status = 404;
			res.set_content("missing fixture", "text/plain");
			return;
		}
		if (origin == "api" && reject_metadata && (route->path == reject_route || req.path == reject_route)) {
			res.set_content("not valid metadata", "application/octet-stream");
			return;
		}
		// Leave successful responses unset so httplib selects 206 for range requests.
		if (route->status != 200) {
			res.status = route->status;
		}
		if (!route->redirect.empty()) {
			res.set_header("Location", route->redirect);
		}
		res.set_content(route->body, "application/octet-stream");
	}

    bool hf_environment = false, had_hf_endpoint = false, had_hf_implicit = false;
    string old_hf_endpoint, old_hf_implicit;
	string provider;
	string auth;
	string reject_route;
	bool require_bearer = false;
	bool require_low = false;
	duckdb_httplib::Server api;
	duckdb_httplib::Server downloads;
	std::thread api_worker;
	std::thread download_worker;
	vector<Route> api_routes;
	vector<Route> download_routes;
	std::mutex trace_mutex;
	vector<Trace> traces;
};

std::mutex fixture_mutex;
map<string, shared_ptr<ProviderFixture>> fixtures;

shared_ptr<ProviderFixture> GetFixture(const string &provider) {
	auto entry = fixtures.find(provider);
	if (entry == fixtures.end()) {
		throw InvalidInputException("Start the provider fixture first: %s", provider);
	}
	return entry->second;
}

string FixtureCommand(const string &provider, const string &command) {
	if (provider.empty() || provider.find_first_not_of(
	                            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-") != string::npos) {
		throw InvalidInputException("Invalid fixture provider name");
	}
	std::lock_guard<std::mutex> guard(fixture_mutex);
	if (command == "start" || command == "start_anonymous" || command == "start_authenticated") {
		fixtures.erase(provider);
		bool authenticated = command == "start_authenticated" || (command == "start" && provider != "internet_archive");
		auto fixture = make_shared_ptr<ProviderFixture>(provider, authenticated);
		auto endpoint = fixture->endpoint;
		fixtures.emplace(provider, std::move(fixture));
		return endpoint;
	}
	if (command == "stop") {
		fixtures.erase(provider);
		return "stopped";
	}
	auto fixture = GetFixture(provider);
	if (command == "download_endpoint") {
		return fixture->download_endpoint;
	}
	if (command == "scope") {
		return fixture->endpoint + fixture->auth_scope;
	}
	if (command == "reject_metadata") {
		fixture->reject_metadata = true;
		return "ok";
	}
	if (command == "reset") {
		fixture->Reset();
		return "ok";
	}
	// Retain the original Internet Archive test API while sharing its server lifecycle.
	if (provider == "internet_archive" && (command == "metadata_requests" || command == "download_requests")) {
		uint64_t count = 0;
		for (const auto &trace : fixture->Snapshot()) {
			if (trace.is_download == (command == "download_requests")) {
				count++;
			}
		}
		return std::to_string(count);
	}
	throw InvalidInputException("Unknown provider fixture command: %s", command);
}

struct RequestsBindData : public FunctionData {
    bool hf_environment = false, had_hf_endpoint = false, had_hf_implicit = false;
    string old_hf_endpoint, old_hf_implicit;
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
	vector<Trace> traces;
	idx_t offset = 0;
};

unique_ptr<GlobalTableFunctionState> RequestsInit(ClientContext &, TableFunctionInitInput &input) {
	auto &bind = input.bind_data->Cast<RequestsBindData>();
	auto result = make_uniq<RequestsState>();
	std::lock_guard<std::mutex> guard(fixture_mutex);
	result->traces = GetFixture(bind.provider)->Snapshot();
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

void LoadInternal(ExtensionLoader &loader) {
	RegisterIcebergFixture(loader);
	RegisterFileAndBlobFixtures(loader);
	ScalarFunction command(
	    "duckport_fixture", {LogicalType::VARCHAR, LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [](DataChunk &args, ExpressionState &, Vector &result) {
		    BinaryExecutor::Execute<string_t, string_t, string_t>(
		        args.data[0], args.data[1], result, args.size(), [&result](string_t provider, string_t input) {
			        return StringVector::AddString(result, FixtureCommand(provider.GetString(), input.GetString()));
		        });
	    });
	command.SetVolatile();
	loader.RegisterFunction(command);
	ScalarFunction ia(
	    "duckport_ia_fixture", {LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [](DataChunk &args, ExpressionState &, Vector &result) {
		    UnaryExecutor::Execute<string_t, string_t>(args.data[0], result, args.size(), [&result](string_t input) {
			    return StringVector::AddString(result, FixtureCommand("internet_archive", input.GetString()));
		    });
	    });
	ia.SetVolatile();
	loader.RegisterFunction(ia);
	TableFunction requests("duckport_fixture_requests", {LogicalType::VARCHAR}, RequestsScan, RequestsBind,
	                       RequestsInit);
	loader.RegisterFunction(requests);
}
} // namespace

void DuckportTestFixtureExtension::Load(ExtensionLoader &loader) {
	LoadInternal(loader);
}
string DuckportTestFixtureExtension::Name() {
	return "duckport_test_fixture";
}
string DuckportTestFixtureExtension::Version() const {
	return "";
}
} // namespace duckdb

extern "C" {
DUCKDB_CPP_EXTENSION_ENTRY(duckport_test_fixture, loader) {
	duckdb::LoadInternal(loader);
}
}
