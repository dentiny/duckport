# Commoncrawl fixtures

A local CC-MAIN-2022-40 mirror with a compressed path manifest and three compressed WARC objects.

Adapted from the MIT-licensed open-arrow-access fixtures.

Four records span three WARC objects. Segment `1664030338280.51` selects two files/three records; `1664030338280.510` selects one metadata record. The shared prefix catches imprecise segment matching. Gzip timestamps are zero.

[Fixture inventory and provenance](../README.md) · [SQL reader tests](../../test/sql/readers/)
