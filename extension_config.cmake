# This file is included by DuckDB's build system. It specifies which extension to load

# Extension from this repo
duckdb_extension_load(duckport
    LOAD_TESTS
    SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}
)

# Any extra extensions that should be built
# e.g.: duckdb_extension_load(json)

# Loopback HTTP fixtures are isolated from the production extension.
option(DUCKPORT_BUILD_TEST_FIXTURES "Build native SQLLogicTest fixtures" ${BUILD_UNITTESTS})
if(DUCKPORT_BUILD_TEST_FIXTURES)
    duckdb_extension_load(duckport_test_fixture
        SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/test/fixture
    )
    include("${CMAKE_CURRENT_LIST_DIR}/duckdb/.github/config/extensions/httpfs.cmake")
endif()
