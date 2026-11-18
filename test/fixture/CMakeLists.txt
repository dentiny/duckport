cmake_minimum_required(VERSION 3.20)
project(duckport_test_fixture)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
include_directories(src/include)
find_package(Threads REQUIRED)
set(FIXTURE_SOURCE
  src/duckport_test_fixture_extension.cpp
  src/fixture_support.cpp
  src/fixture_server.cpp
  src/scoped_fixture_environment.cpp
  src/provider_fixture.cpp
  src/provider_functions.cpp
  src/iceberg.cpp
  src/files_and_blobs.cpp)
build_static_extension(duckport_test_fixture ${FIXTURE_SOURCE})
build_loadable_extension(duckport_test_fixture "" ${FIXTURE_SOURCE})
foreach(FIXTURE_TARGET duckport_test_fixture_extension duckport_test_fixture_loadable_extension)
  target_include_directories(${FIXTURE_TARGET} PRIVATE
    ${CMAKE_SOURCE_DIR}/third_party/httplib
    ${CMAKE_SOURCE_DIR}/third_party/yyjson/include)
  target_compile_definitions(${FIXTURE_TARGET} PRIVATE DUCKPORT_TESTDATA_DIR="${CMAKE_CURRENT_SOURCE_DIR}/../../testdata")
  target_link_libraries(${FIXTURE_TARGET} Threads::Threads duckdb_yyjson duckport_source_rust ${CMAKE_DL_LIBS})
  if(APPLE)
    target_link_libraries(${FIXTURE_TARGET} "-framework Security" "-framework CoreFoundation" "-framework SystemConfiguration" "-framework IOKit")
  elseif(WIN32)
    target_link_libraries(${FIXTURE_TARGET} ws2_32 userenv bcrypt ntdll secur32 crypt32)
  elseif(UNIX)
    target_link_libraries(${FIXTURE_TARGET} m)
  endif()
endforeach()
