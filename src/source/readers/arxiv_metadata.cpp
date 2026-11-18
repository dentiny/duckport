#include "readers/arxiv_metadata.h"
#include "source/options.hpp"
#include "source/readers/arxiv_metadata.hpp"
#include "storage_options.hpp"
namespace duckdb {
char *OpenArxivMetadata(ClientContext &context, const string &locator,
                        const DuckportScanOptions &scan,
                        const TableFunctionBindInput &input,
                        DuckportSource **output) {
  ArxivMetadataOptions reader_options{};
  auto endpoint = ReadStringOption(input, "endpoint");
  reader_options.endpoint = endpoint ? endpoint->c_str() : nullptr;
  auto from_date = ReadStringOption(input, "from_date");
  reader_options.from_date = from_date ? from_date->c_str() : nullptr;
  auto until_date = ReadStringOption(input, "until_date");
  reader_options.until_date = until_date ? until_date->c_str() : nullptr;
  auto set_spec = ReadStringOption(input, "set_spec");
  reader_options.set_spec = set_spec ? set_spec->c_str() : nullptr;
  auto record_path = ReadStringOption(input, "record_path");
  reader_options.record_path = record_path ? record_path->c_str() : nullptr;
  ReadPaginationOptions(input, reader_options.max_pages, reader_options.max_records);
  StorageParameters storage(context,
                            endpoint.value_or("https://oaipmh.arxiv.org") + "/",
                            /*secret_type=*/{});
  return duckport_arxiv_metadata_open(locator.c_str(), storage.config, scan,
                                      reader_options, output);
}
} // namespace duckdb
