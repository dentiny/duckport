#include "provider_fixture.hpp"
#include "fixture_support.hpp"
#include "fixture_server.hpp"
#include "scoped_fixture_environment.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/helper.hpp"
#include "duckdb/common/map.hpp"
#include "duckdb/common/shared_ptr.hpp"
#include "duckdb/common/unique_ptr.hpp"
#include "yyjson.hpp"

#include <atomic>
#include <mutex>

namespace duckdb {
namespace {
using namespace duckdb_yyjson;

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

// Only transport, committed response data, and request evidence live here.
// Expected values and assertions belong in SQLLogicTest files.
class ProviderFixture {
public:
	ProviderFixture(string provider_p, bool authenticated) : provider(std::move(provider_p)) {
		endpoint = api.Endpoint();
		download_endpoint = downloads.Endpoint();
		LoadRoutes();
		require_bearer = authenticated && auth == "authorization";
		require_low = authenticated && provider == "internet_archive";
		RegisterRoutes();
		api.Start();
		downloads.Start();
		ConfigureEnvironment();
	}

	vector<FixtureRequest> Snapshot() {
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
	void RegisterRoutes() {
		api.Http().Post(".*", [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			Respond(req, res, api_routes, "api");
		});
		api.Http().Get(".*", [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			Respond(req, res, api_routes, "api");
		});
		downloads.Http().Get(".*", [this](const duckdb_httplib::Request &req, duckdb_httplib::Response &res) {
			Respond(req, res, download_routes, "download");
		});
	}

	void ConfigureEnvironment() {
		if (provider == "blob_hf") {
			environment.push_back(make_uniq<ScopedFixtureEnvironment>("HF_ENDPOINT", endpoint.c_str()));
			environment.push_back(make_uniq<ScopedFixtureEnvironment>("HF_HUB_DISABLE_IMPLICIT_TOKEN", "1"));
		}
	}

	string Expand(string value) {
		ReplaceAll(value, "API_ORIGIN", endpoint);
		ReplaceAll(value, "ORIGIN", download_endpoint);
		return value;
	}

	Route ParseRoute(yyjson_val *entry, bool foreign) {
		Route route;
		route.path = JsonString(entry, "path");
		route.file = JsonString(entry, "file");
		if (route.path.empty() || route.file.empty()) {
			throw InvalidInputException("Fixture route requires path and file");
		}
		route.body = Expand(ReadFixtureFile(provider, route.file));
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
		return route;
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
			routes.push_back(ParseRoute(entry, foreign));
		}
	}

	void LoadRoutes() {
		auto json = ReadFixtureFile(provider, "http.json");
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

	static bool RejectRequest(duckdb_httplib::Response &res, int status, const char *message) {
		res.status = status;
		res.set_content(message, "text/plain");
		return false;
	}

	void RecordRequest(const duckdb_httplib::Request &req, const Route *route, const string &origin) {
		string headers;
		for (const auto &header : req.headers) {
			headers += header.first + ": " + header.second + "\n";
		}
		FixtureRequest trace {origin,
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
	}

	bool Authorize(const duckdb_httplib::Request &req, duckdb_httplib::Response &res, const Route *route,
	               const string &origin) {
		const auto expected_token = provider == "figshare" ? "token fixture-key" : "Bearer fixture-key";
		if (origin == "api" && require_bearer && req.get_header_value("Authorization") != expected_token) {
			return RejectRequest(res, 401, "authentication required");
		}
		if (origin == "api" && require_low &&
		    req.get_header_value("Authorization") != "LOW fixture-key:fixture-secret") {
			return RejectRequest(res, 401, "Internet Archive credentials required");
		}
		if (route && !route->auth_header.empty() &&
		    (route->auth_prefix ? req.get_header_value(route->auth_header).rfind(route->auth_value, 0) != 0
		                        : req.get_header_value(route->auth_header) != route->auth_value)) {
			return RejectRequest(res, 401, "source credentials required");
		}
		if (route && !route->session_token.empty() &&
		    req.get_header_value("x-amz-security-token") != route->session_token) {
			return RejectRequest(res, 403, "source session token required");
		}
		return true;
	}

	void Respond(const duckdb_httplib::Request &req, duckdb_httplib::Response &res, const vector<Route> &routes,
	             const string &origin) {
		auto route = FindRoute(routes, req);
		RecordRequest(req, route, origin);
		if (!Authorize(req, res, route, origin)) {
			return;
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

	vector<unique_ptr<ScopedFixtureEnvironment>> environment;
	string provider;
	string auth;
	string reject_route;
	bool require_bearer = false;
	bool require_low = false;
	vector<Route> api_routes;
	vector<Route> download_routes;
	std::mutex trace_mutex;
	vector<FixtureRequest> traces;
	// Stop workers before destroying any state captured by request handlers.
	FixtureServer api {"provider fixture servers"};
	FixtureServer downloads {"provider fixture servers"};
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

string StartProviderFixture(const string &provider, const string &command) {
	fixtures.erase(provider);
	bool authenticated = command == "start_authenticated" || (command == "start" && provider != "internet_archive");
	auto fixture = make_shared_ptr<ProviderFixture>(provider, authenticated);
	auto endpoint = fixture->endpoint;
	fixtures.emplace(provider, std::move(fixture));
	return endpoint;
}

string QueryProviderFixture(ProviderFixture &fixture, const string &provider, const string &command) {
	if (command == "download_endpoint") {
		return fixture.download_endpoint;
	}
	if (command == "scope") {
		return fixture.endpoint + fixture.auth_scope;
	}
	if (command == "reject_metadata") {
		fixture.reject_metadata = true;
		return "ok";
	}
	if (command == "reset") {
		fixture.Reset();
		return "ok";
	}
	// Retain the original Internet Archive test API while sharing its server lifecycle.
	if (provider == "internet_archive" && (command == "metadata_requests" || command == "download_requests")) {
		uint64_t count = 0;
		for (const auto &trace : fixture.Snapshot()) {
			if (trace.is_download == (command == "download_requests")) {
				count++;
			}
		}
		return std::to_string(count);
	}
	throw InvalidInputException("Unknown provider fixture command: %s", command);
}
} // namespace

string ProviderFixtureCommand(const string &provider, const string &command) {
	if (provider.empty() || provider.find_first_not_of(
	                            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-") != string::npos) {
		throw InvalidInputException("Invalid fixture provider name");
	}
	std::lock_guard<std::mutex> guard(fixture_mutex);
	if (command == "start" || command == "start_anonymous" || command == "start_authenticated") {
		return StartProviderFixture(provider, command);
	}
	if (command == "stop") {
		fixtures.erase(provider);
		return "stopped";
	}
	return QueryProviderFixture(*GetFixture(provider), provider, command);
}

vector<FixtureRequest> ProviderFixtureRequests(const string &provider) {
	std::lock_guard<std::mutex> guard(fixture_mutex);
	return GetFixture(provider)->Snapshot();
}
} // namespace duckdb
