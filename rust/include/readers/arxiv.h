#pragma once
#include "duckport.h"

struct ArxivOptions {
	const char *kind;
	const char *format;
	const char *article_id;
	const char *month;
	const char *shard;
	int32_t requester_pays;
	const char *metadata_uri;
	const char *metadata_id_column;
	const char *metadata_endpoint;
	const DuckportStorageOptions *metadata_storage;
};

extern "C" {
char *duckport_arxiv_open(const char *locator, const DuckportStorageOptions &storage_options,
                          const DuckportScanOptions &scan_options, const ArxivOptions &reader_options,
                          DuckportSource **output);
}
