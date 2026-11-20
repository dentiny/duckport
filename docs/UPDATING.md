# Development

This guide is for contributors building a local extension. To install the
community package, follow the [README](../README.md#start-here).

## Build and load

Use CMake 3.20+, a C++17 toolchain, Rust with edition-2024 support compatible
with `rust/Cargo.lock`, and `protoc` for the Lance dependency. Initialize the
pinned DuckDB and extension-ci-tools submodules before building:

```sh
git submodule update --init --recursive
make
./build/release/duckdb
```

The build's shell includes DuckPort. A separately launched shell must match the
pinned DuckDB revision and permit an unsigned local extension (`-unsigned`):

```sql
LOAD 'build/release/extension/duckport/duckport.duckdb_extension';
```

Use `make EXT_FLAGS='-DDUCKPORT_CARGO_PROFILE=dev'` for the Cargo dev profile;
release is the default. Cross builds require a matching DUCKPORT_RUST_TARGET (or
Rust_CARGO_TARGET). Build macOS architectures separately. Build paths and flags
are controlled by CMake and extension-ci-tools.

## Source layout

| Path | Responsibility |
| --- | --- |
| `src/source/`, `src/include/source/` | DuckDB scan registration, typed options and Arrow scan factory |
| `src/sink/`, `src/include/sink/` | COPY option binding, query transformations and backend openers |
| `src/storage_options.cpp`, `src/security_options.cpp`, `src/blob_secrets.cpp` | Session policy and scoped secret capture |
| `rust/src/ffi/` | Shared runtime, reader declaration macro, handles and Arrow ABI operations |
| `rust/src/readers/`, `rust/src/source.rs` | Provider adapters and upstream PreparedSource binding |
| `rust/src/sink/` | Backend configuration and writer lifecycle |
| `rust/src/validation.rs`, `rust/src/utils.rs` | Input checks and boundary conversions |
| `test/sql/`, `test/fixture/`, `testdata/` | SQL assertions, native fixture support and payloads |

Read the [architecture guide](architecture.md) before adding functionality.

## Add or change a reader

Reuse open-arrow-access accessors/requests rather than implementing format or
service protocols in the extension. Declare ordinary C entry points through
`export_reader!`; keep the provider adapter expression visible at its call site.
Typed reader options belong in the provider's own C header and Rust repr(C)
struct. Common scan options currently contain only batch_size.

C++ resolves each storage URI's secrets on the query thread. Separate URI
namespaces require separate StorageParameters, including arXiv metadata snapshots.
Copy borrowed options into owned Rust state during open. Update both language
layouts and registrations together. Keep SQL tests focused on end-to-end data
behavior. Do not add SQL configuration matrices that repeat option validation
or checks already owned by the upstream crates.

## Add or change a sink

Reuse open-arrow-sink's core, layer, storage and backend capabilities. Keep
backend setup in its own module; common COPY planning/options do not need a
format-name switch. New reusable storage/schema/blob machinery belongs in the
upstream projects. Backend differences must be reflected in the
[COPY capability table](writers/README.md#backend-capabilities).

Keep functions focused; extract common operations when they remove duplication.
A new class can have its own files. Avoid one-use forwarding wrappers and
framework layers that add more maintenance than the code they replace. Use
DuckDB common containers, strings, atomics, threads and annotated mutexes when
available. Annotate guarded state and lock requirements. Resource-owning members
should clean up in their destructors, with workers destroyed before the state
captured by their handlers.

## Dependencies and verification

The manifest currently selects open-arrow-access 0.12.1, open-arrow-sink 0.6.2
and Arrow 58. Check both upstream release notes and the pinned Rust APIs when
upgrading. Update Cargo.toml/Cargo.lock together and keep the C ABI in sync.
A DuckDB upgrade also requires reviewing internal C++ headers and updating the
DuckDB/extension-ci-tools submodule pins.

Install clang-format 11.0.1 (matching the pinned DuckDB formatter), cmake-format,
Taplo 0.10.0 and the Rust formatting component. From the repository root:

```sh
make format-all        # Apply CMake, C/C++, TOML and Rust formatting.
make format-check      # Check those formats without changing files.
make toml-format-check # Check only rust/Cargo.toml.
git diff --check
```

`make format` applies CMake and C/C++ formatting; `make format-fix` aliases
`format-all`. Formatter executables can be selected with `CLANG_FORMAT`,
`CMAKE_FORMAT` and `TAPLO`, for example `make format-check TAPLO=/path/to/taplo`.
Formatting-only targets do not load the build Makefile and work without initialized
build submodules. The standalone `.clang-format` matches the pinned DuckDB style;
keep it aligned when updating that pin.

These commands do not compile. `make` and `make test` can compile: run them only
when builds are allowed. [Test setup](../test/README.md) describes loopback and
independent-reader requirements. Documentation changes also need local-link,
SQL function/option-name and capability checks against implementation/tests.
Taplo configuration is scoped to this repository's Cargo manifest, excluding
submodules; `make toml-format` applies it and `make toml-format-check` checks it.
