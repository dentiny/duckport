#include "readers/arxiv.h"
#include "source/options.hpp"
#include "source/readers/arxiv.hpp"
#include "storage_options.hpp"
namespace duckdb {
char *OpenArxiv(ClientContext &context, const string &locator, const DuckportScanOptions &scan,
                const TableFunctionBindInput &input, DuckportSource **output) {
	ArxivOptions reader_options {};
	auto kind = ReadStringOption(input, "kind");
	reader_options.kind = kind ? kind->c_str() : nullptr;
	auto format = ReadStringOption(input, "format");
	reader_options.format = format ? format->c_str() : nullptr;
	auto article_id = ReadStringOption(input, "article_id");
	reader_options.article_id = article_id ? article_id->c_str() : nullptr;
	auto month = ReadStringOption(input, "month");
	reader_options.month = month ? month->c_str() : nullptr;
	auto shard = ReadStringOption(input, "shard");
	reader_options.shard = shard ? shard->c_str() : nullptr;
	ReadBooleanOption(input, "requester_pays", reader_options.requester_pays);
	auto metadata_endpoint = ReadStringOption(input, "metadata_endpoint");
	reader_options.metadata_endpoint = metadata_endpoint ? metadata_endpoint->c_str() : nullptr;
	auto metadata_uri = ReadStringOption(input, "metadata_uri");
	auto metadata_id_column = ReadStringOption(input, "metadata_id_column");
	reader_options.metadata_uri = metadata_uri ? metadata_uri->c_str() : nullptr;
	reader_options.metadata_id_column = metadata_id_column ? metadata_id_column->c_str() : nullptr;
	unique_ptr<StorageParameters> metadata_storage;
	if (metadata_uri) {
		metadata_storage = make_uniq<StorageParameters>(context, *metadata_uri, string());
		reader_options.metadata_storage = &metadata_storage->config;
	}
	StorageParameters storage(context, locator, /*secret_type=*/ {});
	return duckport_arxiv_open(locator.c_str(), storage.config, scan, reader_options, output);
}
} // namespace duckdb
