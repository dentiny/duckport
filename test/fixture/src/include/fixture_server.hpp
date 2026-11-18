#pragma once

#include "duckdb/common/string.hpp"
#include "httplib.hpp"
#include "duckdb/common/thread.hpp"

namespace duckdb {
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

private:
	void Stop();
	duckdb_httplib::Server server;
	thread worker;
	string description;
	string endpoint;
};

} // namespace duckdb
