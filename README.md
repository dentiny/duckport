# DuckPort

A DuckDB extension for connecting diverse data sources to DuckDB and writing to analytical storage.

DuckPort is in early development. The planned pipeline is:

```text
Sources → open-arrow-access → Arrow → DuckDB SQL → Iceberg / Lance / Parquet
```

Source integration through [open-arrow-access](https://github.com/dentiny/open-arrow-access) and sink support are planned; the current implementation contains only the renamed DuckDB extension template functions.

## Build

Clone with submodules:

```sh
git clone --recurse-submodules https://github.com/dentiny/duckport.git
cd duckport
```

The template currently depends on OpenSSL. Install its development libraries so CMake can find them, or configure vcpkg using `VCPKG_TOOLCHAIN_PATH`.

```sh
make
make test
```

Build outputs include:

- `build/release/duckdb`
- `build/release/test/unittest`
- `build/release/extension/duckport/duckport.duckdb_extension`

## Try the template functions

The built DuckDB shell includes DuckPort:

```sh
./build/release/duckdb
```

```sql
SELECT duckport('Sam');
-- ...........🦆 Sam

SELECT duckport_openssl_version('Sam');
```

To load the built extension into a compatible DuckDB shell, start it with unsigned extensions enabled (`duckdb -unsigned`) and run:

```sql
LOAD 'build/release/extension/duckport/duckport.duckdb_extension';
```

DuckPort is based on [duckdb/extension-template](https://github.com/duckdb/extension-template). See [docs/UPDATING.md](docs/UPDATING.md) for template update instructions.
