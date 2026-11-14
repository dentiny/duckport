#pragma once

#include "duckport.h"

struct XmlOptions {
	const char *record_path;
};

extern "C" {
char *duckport_xml_open(const char *locator, const StorageOptions *storage_options, const ScanOptions *scan_options,
						const XmlOptions *reader_options, DuckportSource **output);
}
