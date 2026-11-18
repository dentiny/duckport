#define DUCKDB_EXTENSION_MAIN
#include "duckport_extension.hpp"
#include "sink/copy.hpp"
#include "storage_options.hpp"
#include "readers/commoncrawl.h"
#include "readers/json.h"
#include "readers/warc.h"
#include "source/reader.hpp"
#include "source/readers/oapen.hpp"
#include "source/readers/doab.hpp"
#include "source/readers/arxiv.hpp"
#include "source/readers/arxiv_metadata.hpp"
#include "readers/meca.h"
#include "source/readers/courtlistener.hpp"
#include "source/readers/biorxiv.hpp"
#include "source/readers/uspto.hpp"
#include "source/readers/regulations.hpp"
#include "source/readers/bhl.hpp"
#include "source/readers/openstax.hpp"
#include "source/readers/semantic_scholar.hpp"
#include "source/readers/library_of_congress.hpp"
#include "source/readers/hathifiles.hpp"
#include "source/readers/govinfo.hpp"
#include "source/readers/federal_register.hpp"
#include "source/readers/internet_archive.hpp"
#include "source/readers/figshare.hpp"
#include "source/readers/dataverse.hpp"
#include "source/readers/zenodo.hpp"
#include "readers/pmc.h"
#include "readers/gutenberg.h"
#include "readers/atif.h"
#include "source/readers/csv.hpp"
#include "source/readers/huggingface.hpp"
#include "source/readers/xml.hpp"

namespace duckdb {
namespace {
void LoadInternal(ExtensionLoader &loader) {
	RegisterStorageSettings(loader);
	RegisterCopyFunctions(loader);
	RegisterReader(loader, "duckport_scan_oapen", OpenOapen, [](TypedKwargs &options) {
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("from_date", LogicalType::VARCHAR);
		options.Add("until_date", LogicalType::VARCHAR);
		options.Add("set_spec", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("max_pages", LogicalType::BIGINT);
		options.Add("max_records", LogicalType::BIGINT);
	});
	RegisterReader(loader, "duckport_scan_doab", OpenDoab, [](TypedKwargs &options) {
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("from_date", LogicalType::VARCHAR);
		options.Add("until_date", LogicalType::VARCHAR);
		options.Add("set_spec", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("max_pages", LogicalType::BIGINT);
		options.Add("max_records", LogicalType::BIGINT);
	});
	RegisterReader(loader, "duckport_scan_arxiv_metadata", OpenArxivMetadata, [](TypedKwargs &options) {
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("from_date", LogicalType::VARCHAR);
		options.Add("until_date", LogicalType::VARCHAR);
		options.Add("set_spec", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("max_pages", LogicalType::BIGINT);
		options.Add("max_records", LogicalType::BIGINT);
	});
	RegisterReader(loader, "duckport_scan_arxiv", OpenArxiv, [](TypedKwargs &options) {
		options.Add("kind", LogicalType::VARCHAR);
		options.Add("month", LogicalType::VARCHAR);
		options.Add("requester_pays", LogicalType::BOOLEAN);
		options.Add("metadata_uri", LogicalType::VARCHAR);
		options.Add("metadata_id_column", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_meca", duckport_meca_open);
	RegisterReader(loader, "duckport_scan_courtlistener", OpenCourtlistener, [](TypedKwargs &options) {
		options.Add("table_name", LogicalType::VARCHAR);
		options.Add("date", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_biorxiv", OpenBiorxiv, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("until", LogicalType::VARCHAR);
		options.Add("server", LogicalType::VARCHAR);
		options.Add("cursor", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_uspto", OpenUspto, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("offset", LogicalType::VARCHAR);
		options.Add("page_size", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_regulations", OpenRegulations, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("resource", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_bhl", OpenBhl, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_openstax", OpenOpenstax, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("revision", LogicalType::VARCHAR);
		options.Add("archive_endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_semantic_scholar", OpenSemanticScholar, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("release", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_library_of_congress", OpenLibraryOfCongress, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_hathifiles", OpenHathifiles, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_govinfo", OpenGovinfo, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_federal_register", OpenFederalRegister, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_internet_archive", OpenInternetArchive, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_figshare", OpenFigshare, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("version", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_dataverse", OpenDataverse, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
		options.Add("version", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_zenodo", OpenZenodo, [](TypedKwargs &options) {
		options.Add("file_name", LogicalType::VARCHAR);
		options.Add("format", LogicalType::VARCHAR);
		options.Add("record_path", LogicalType::VARCHAR);
		options.Add("endpoint", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_pmc", duckport_pmc_open);
	RegisterReader(loader, "duckport_scan_pmc_inventory", duckport_pmc_inventory_open);
	RegisterReader(loader, "duckport_scan_gutenberg", duckport_gutenberg_open);
	RegisterReader(loader, "duckport_scan_parquet", duckport_parquet_source_open);
	RegisterReader(loader, "duckport_scan_warc", duckport_warc_open);
	RegisterReader(loader, "duckport_scan_commoncrawl", duckport_commoncrawl_open);
	RegisterReader(loader, "duckport_scan_commoncrawl_segment", duckport_commoncrawl_segment_open);
	RegisterReader(loader, "duckport_scan_atif", duckport_atif_open);
	auto add_csv_options = [](TypedKwargs &options) {
		options.Add("header", LogicalType::BOOLEAN);
	};
	RegisterReader(loader, "duckport_scan_csv", OpenCsv, [](TypedKwargs &options) {
		options.Add("header", LogicalType::BOOLEAN);
		options.Add("delimiter", LogicalType::VARCHAR);
	});
	RegisterReader(loader, "duckport_scan_tsv", OpenTsv, add_csv_options);
	RegisterReader(loader, "duckport_scan_json", duckport_json_open);
	RegisterReader(loader, "duckport_scan_jsonl", duckport_json_open);
	RegisterReader(loader, "duckport_scan_ndjson", duckport_json_open);
	RegisterReader(loader, "duckport_scan_xml", OpenXml,
	               [](TypedKwargs &options) { options.Add("record_path", LogicalType::VARCHAR); });
	RegisterReader(loader, "duckport_scan_huggingface", OpenHuggingFace, [](TypedKwargs &options) {
		options.Add("revision", LogicalType::VARCHAR);
		options.Add("config", LogicalType::VARCHAR);
		options.Add("split", LogicalType::VARCHAR);
		options.Add("allow_partial", LogicalType::BOOLEAN);
	});
}
} // namespace
void DuckportExtension::Load(ExtensionLoader &loader) {
	LoadInternal(loader);
}
string DuckportExtension::Name() {
	return "duckport";
}
string DuckportExtension::Version() const {
#ifdef EXT_VERSION_DUCKPORT
	return EXT_VERSION_DUCKPORT;
#else
	return "";
#endif
}
} // namespace duckdb
extern "C" {
DUCKDB_CPP_EXTENSION_ENTRY(duckport, loader) {
	duckdb::LoadInternal(loader);
}
}
