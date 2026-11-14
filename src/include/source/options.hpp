#pragma once

#include "duckdb.hpp"
#include "duckport.h"

namespace duckdb {
string ReadSourceString(const Value &value);

class ScanParameters {
  public:
	explicit ScanParameters(const TableFunctionBindInput &input);
	ScanParameters(const ScanParameters &) = delete;
	ScanParameters &operator=(const ScanParameters &) = delete;
	::ScanOptions scan{};

  private:
	string revision;
	string record_path;
	string format;
	string path;
	string config;
	string split;
};
} // namespace duckdb
