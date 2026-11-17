PROJ_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# Configuration of extension
EXT_NAME=duckport
# All DuckPort SQLLogicTests live under test/sql/.
TESTS_BASE_DIRECTORY=test/sql/
EXT_CONFIG=${PROJ_DIR}extension_config.cmake

# Include the Makefile from extension-ci-tools
include extension-ci-tools/makefiles/duckdb_extension.Makefile

.PHONY: toml-format toml-format-check

toml-format:
	taplo fmt

toml-format-check:
	taplo fmt --check

DUCKPORT_TEST_WRITER ?= build/release/duckdb
DUCKPORT_TEST_EXTENSION ?= build/release/extension/duckport/duckport.duckdb_extension
DUCKPORT_TEST_READER ?= duckdb

.PHONY: test-iceberg

test-iceberg:
	python3 test/integration/iceberg_roundtrip.py \
		--writer "$(DUCKPORT_TEST_WRITER)" \
		--duckport-extension "$(DUCKPORT_TEST_EXTENSION)" \
		--reader "$(DUCKPORT_TEST_READER)"
