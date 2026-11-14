SQL integration fixtures adapted from the MIT-licensed
[open-arrow-access service tests](https://github.com/dentiny/open-arrow-access/tree/v0.1.0/service/tests)
and corresponding `testdata/gutenberg` fixtures in version 0.1.0.

HTTP fixtures retain provider response shapes and replace download origins with
local servers. Small CSV payloads replace publication content where appropriate.
The arXiv TAR fixture is generated deterministically using the upstream manifest shape.
