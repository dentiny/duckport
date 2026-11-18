PROJ_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# Configuration of extension
EXT_NAME=duckport
# All DuckPort SQLLogicTests live under test/sql/.
TESTS_BASE_DIRECTORY=test/sql/
EXT_CONFIG=${PROJ_DIR}extension_config.cmake

# Formatting must work without loading the build system or initializing submodules.
FORMAT_TARGETS := format format-all format-check format-fix toml-format toml-format-check
ifeq ($(filter-out $(FORMAT_TARGETS),$(or $(MAKECMDGOALS),all)),)
CMAKE_FILES := CMakeLists.txt extension_config.cmake test/fixture/CMakeLists.txt
CPP_FILES := $(shell find src rust/include test/fixture/src -type f \( -name '*.cpp' -o -name '*.hpp' -o -name '*.h' \))
CLANG_FORMAT ?= clang-format
CMAKE_FORMAT ?= cmake-format
TAPLO ?= taplo

.PHONY: $(FORMAT_TARGETS)

format:
	$(CMAKE_FORMAT) -i $(CMAKE_FILES)
	$(CLANG_FORMAT) -i $(CPP_FILES)

format-all: format toml-format
	cargo fmt --manifest-path rust/Cargo.toml

format-fix: format-all

format-check:
	$(CMAKE_FORMAT) --check $(CMAKE_FILES)
	$(CLANG_FORMAT) --dry-run --Werror $(CPP_FILES)
	cargo fmt --manifest-path rust/Cargo.toml -- --check
	$(TAPLO) fmt --check

toml-format:
	$(TAPLO) fmt

toml-format-check:
	$(TAPLO) fmt --check
else
include extension-ci-tools/makefiles/duckdb_extension.Makefile
endif
