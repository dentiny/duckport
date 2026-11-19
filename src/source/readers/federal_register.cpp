#include "readers/federal_register.h"
#include "source/readers/attachment.hpp"
#include "source/readers/federal_register.hpp"
#include "storage_options.hpp"

namespace duckdb {
char *OpenFederalRegister(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                          const TableFunctionBindInput &input, DuckportSource **output) {
	FederalRegisterOptions reader_options {};
	AttachmentOptionsFfiView common(input, reader_options);
	StorageParameters storage(context, common.Endpoint().value_or("https://www.federalregister.gov") + "/",
	                          /*secret_type=*/ {});
	return duckport_federal_register_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
