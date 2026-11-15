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
