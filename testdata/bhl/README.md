# Bhl fixtures

An item metadata response and selected text attachment, including native API-key query authentication.

Adapted from the MIT-licensed [open-arrow-access v0.1.0 service tests](https://github.com/dentiny/open-arrow-access/tree/v0.1.0/service/tests) and its `bhl` fixture data. Local payloads are reduced/synthetic adaptations.

[http.json](http.json) defines response routes and synthetic authentication. Route syntax and loopback commands are documented in the [test guide](../../test/README.md#native-http-fixtures).

[Fixture inventory and provenance](../README.md) · [SQL reader tests](../../test/sql/readers/)

`open-data/` contains three synthetic OCR objects: a text page (11 bytes), a
blank CRLF page (2 bytes), and an unselected object. The Open Data SQL test selects
only the first two through a local mirror with concurrent native partitions.
