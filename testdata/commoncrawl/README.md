# Common Crawl fixtures

`warc.paths` is copied from open-arrow-access's Common Crawl fixture. Its gzip copy and the three compressed WARC files form a small local mirror of `CC-MAIN-2022-40`. WARC content comes from the upstream WARC fixtures; gzip timestamps are fixed to zero.

The dataset has four records. Segment `1664030338280.51` selects two files with three records; `1664030338280.510` selects one file with one metadata record. This checks exact segment selection, including the shared name prefix.
