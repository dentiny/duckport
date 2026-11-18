#include "fixture_server.hpp"
#include "duckdb/common/exception.hpp"

namespace duckdb {
FixtureServer::FixtureServer(const string &description) : description(description) {
	auto port = server.bind_to_any_port("127.0.0.1");
	if (port < 0) {
		throw IOException("Cannot bind %s", description);
	}
	endpoint = "http://127.0.0.1:" + std::to_string(port);
}

FixtureServer::~FixtureServer() {
	Stop();
}

duckdb_httplib::Server &FixtureServer::Http() {
	return server;
}

const string &FixtureServer::Endpoint() const {
	return endpoint;
}

void FixtureServer::Start() {
	worker = std::thread([this] { server.listen_after_bind(); });
	server.wait_until_ready();
	if (!server.is_running()) {
		Stop();
		throw IOException("Cannot start %s", description);
	}
}

void FixtureServer::Stop() {
	server.stop();
	if (worker.joinable()) {
		worker.join();
	}
}

} // namespace duckdb
