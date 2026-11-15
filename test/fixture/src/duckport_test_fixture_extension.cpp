#define DUCKDB_EXTENSION_MAIN
#include "duckport_test_fixture_extension.hpp"
#include "duckdb/common/vector_operations/unary_executor.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "httplib.hpp"

#include <atomic>
#include <fstream>
#include <iterator>
#include <mutex>
#include <thread>

namespace duckdb {
namespace {
std::string ReadFixture(const char *name) {
	std::ifstream file(std::string(DUCKPORT_IA_FIXTURE_DIR) + "/" + name, std::ios::binary);
	if (!file) {
		throw IOException("Cannot open Internet Archive fixture: %s", name);
	}
	return std::string(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
}

// This helper is a separate test extension: no fixture functions enter Duckport builds.
class InternetArchiveFixture {
public:
	InternetArchiveFixture() {
		auto metadata = ReadFixture("metadata.json");
		auto attachment = ReadFixture("attachment.bin");
		server.Get("/metadata/sample-item",
		           [this, metadata](const duckdb_httplib::Request &, duckdb_httplib::Response &res) {
			           metadata_requests++;
			           res.set_content(reject_metadata ? "not valid metadata" : metadata, "application/json");
		           });
		// cpp-httplib decodes percent-encoded paths and implements HEAD and byte ranges.
		server.Get("/download/sample-item/papers/paper #1%.pdf",
		           [this, attachment](const duckdb_httplib::Request &, duckdb_httplib::Response &res) {
			           download_requests++;
			           res.set_content(attachment, "application/octet-stream");
		           });
		auto port = server.bind_to_any_port("127.0.0.1");
		if (port < 0) {
			throw IOException("Cannot bind Internet Archive HTTP fixture");
		}
		endpoint = "http://127.0.0.1:" + std::to_string(port);
		worker = std::thread([this] { server.listen_after_bind(); });
		server.wait_until_ready();
		if (!server.is_running()) {
			worker.join();
			throw IOException("Cannot start Internet Archive HTTP fixture");
		}
	}
	~InternetArchiveFixture() {
		server.stop();
		if (worker.joinable()) {
			worker.join();
		}
	}
	duckdb_httplib::Server server;
	std::thread worker;
	std::string endpoint;
	std::atomic<bool> reject_metadata {false};
	std::atomic<uint64_t> metadata_requests {0};
	std::atomic<uint64_t> download_requests {0};
};

std::mutex fixture_mutex;
std::unique_ptr<InternetArchiveFixture> fixture;

std::string FixtureCommand(const std::string &command) {
	std::lock_guard<std::mutex> lock(fixture_mutex);
	if (command == "start") {
		fixture.reset();
		fixture = std::make_unique<InternetArchiveFixture>();
		return fixture->endpoint;
	}
	if (command == "stop") {
		fixture.reset();
		return "stopped";
	}
	if (!fixture) {
		throw InvalidInputException("Start the Internet Archive fixture first");
	}
	if (command == "reject_metadata") {
		fixture->reject_metadata = true;
		return "ok";
	}
	if (command == "metadata_requests") {
		return std::to_string(fixture->metadata_requests.load());
	}
	if (command == "download_requests") {
		return std::to_string(fixture->download_requests.load());
	}
	throw InvalidInputException("Unknown Internet Archive fixture command: %s", command);
}

void LoadInternal(ExtensionLoader &loader) {
	ScalarFunction command("duckport_ia_fixture", {LogicalType::VARCHAR}, LogicalType::VARCHAR,
	                       [](DataChunk &args, ExpressionState &, Vector &result) {
		                       UnaryExecutor::Execute<string_t, string_t>(
		                           args.data[0], result, args.size(), [&result](string_t input) {
			                           return StringVector::AddString(result, FixtureCommand(input.GetString()));
		                           });
	                       });
	command.SetVolatile();
	loader.RegisterFunction(command);
}
} // namespace
void DuckportTestFixtureExtension::Load(ExtensionLoader &loader) {
	LoadInternal(loader);
}
std::string DuckportTestFixtureExtension::Name() {
	return "duckport_test_fixture";
}
std::string DuckportTestFixtureExtension::Version() const {
	return "";
}
} // namespace duckdb

extern "C" {
DUCKDB_CPP_EXTENSION_ENTRY(duckport_test_fixture, loader) {
	duckdb::LoadInternal(loader);
}
}
