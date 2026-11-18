#pragma once

#include "duckdb/common/string.hpp"
#include "duckdb/main/extension/extension_loader.hpp"
#include "httplib.hpp"
#include <functional>
#include <thread>

namespace duckdb {
string ReadFixtureFile(const string &provider, const string &name);

// Own the listener and its worker so constructor failures also stop the server.
class FixtureServer {
public:
	explicit FixtureServer(const string &description);
	~FixtureServer();
	FixtureServer(const FixtureServer &) = delete;
	FixtureServer &operator=(const FixtureServer &) = delete;

	duckdb_httplib::Server &Http();
	const string &Endpoint() const;
	void Start();
	void Stop();

private:
	duckdb_httplib::Server server;
	std::thread worker;
	string description;
	string endpoint;
};

// Restore each process setting when its fixture shuts down, including on error.
class ScopedFixtureEnvironment {
public:
	ScopedFixtureEnvironment(const char *name, const char *value);
	~ScopedFixtureEnvironment();
	ScopedFixtureEnvironment(const ScopedFixtureEnvironment &) = delete;
	ScopedFixtureEnvironment &operator=(const ScopedFixtureEnvironment &) = delete;

private:
	string name;
	string previous;
	bool was_set;
};

void RegisterBinaryFixtureCommand(ExtensionLoader &loader, const string &name,
                                  std::function<string(const string &, const string &)> command);
void RegisterProviderFixtures(ExtensionLoader &loader);
void RegisterFileAndBlobFixtures(ExtensionLoader &loader);
} // namespace duckdb
