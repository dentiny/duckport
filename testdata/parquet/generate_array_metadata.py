"""Regenerate the fixed-size array metadata fixture (requires pyarrow)."""
from pathlib import Path

import pyarrow as pa
import pyarrow.parquet as pq


def tagged(name, data_type, policy, document):
    return pa.field(
        name,
        data_type,
        metadata={
            "duckport:storage_policy": policy,
            "duckport:media_document": document,
        },
    )


schema = pa.schema(
    [
        pa.field(
            "values",
            pa.list_(tagged("element", pa.string(), "external", "file:///array.json"), 2),
        ),
        pa.field(
            "records",
            pa.list_(
                pa.struct(
                    [tagged("payload", pa.string(), "inline", "file:///nested-array.json")]
                ),
                2,
            ),
        ),
    ]
)
table = pa.Table.from_pylist(
    [{"values": ["first", "second"], "records": [{"payload": "nested"}, None]}],
    schema=schema,
)
pq.write_table(table, Path(__file__).with_name("array_metadata.parquet"))
