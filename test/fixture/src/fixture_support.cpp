#include "fixture_support.hpp"
#include "duckdb/common/exception.hpp"
#include "duckdb/common/vector_operations/binary_executor.hpp"
#include "duckdb/function/scalar_function.hpp"
#include <cstdlib>
#include <fstream>
#include <iterator>

namespace duckdb {
namespace {
void SetFixtureEnvironment(const char *name, const char *value) {
#ifdef _WIN32
	_putenv_s(name, value ? value : "");
#else
	if (value) {
		setenv(name, value, 1);
	} else {
		unsetenv(name);
	}
#endif
}
} // namespace

string ReadFixtureFile(const string &provider, const string &name) {
	std::ifstream file(string(DUCKPORT_TESTDATA_DIR) + "/" + provider + "/" + name, std::ios::binary);
	if (!file) {
		throw IOException("Cannot open %s fixture: %s", provider, name);
	}
	return string(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
}

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

ScopedFixtureEnvironment::ScopedFixtureEnvironment(const char *name, const char *value) : name(name) {
	auto old = std::getenv(name);
	was_set = old != nullptr;
	if (old) {
		previous = old;
	}
	SetFixtureEnvironment(name, value);
}

ScopedFixtureEnvironment::~ScopedFixtureEnvironment() {
	SetFixtureEnvironment(name.c_str(), was_set ? previous.c_str() : nullptr);
}

void RegisterBinaryFixtureCommand(ExtensionLoader &loader, const string &name,
                                  std::function<string(const string &, const string &)> command) {
	ScalarFunction function(
	    Identifier(name), {LogicalType::VARCHAR, LogicalType::VARCHAR}, LogicalType::VARCHAR,
	    [command](DataChunk &args, ExpressionState &, Vector &result) {
		    BinaryExecutor::Execute<string_t, string_t, string_t>(
		        args.data[0], args.data[1], result, args.size(), [&result, &command](string_t first, string_t second) {
			        return StringVector::AddString(result, command(first.GetString(), second.GetString()));
		        });
	    });
	function.SetVolatile();
	loader.RegisterFunction(function);
}
} // namespace duckdb
