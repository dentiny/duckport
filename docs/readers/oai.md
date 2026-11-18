# OAI-PMH metadata

`duckport_scan_doab`, `duckport_scan_oapen` and
`duckport_scan_arxiv_metadata` automatically follow continuation tokens.

```sql
SELECT identifier, datestamp, status, set_specs
FROM duckport_scan_doab('records', from_date := '2026-01-01',
    until_date := '2026-01-31', max_pages := 100, max_records := 10000);
```

| Function | Default endpoint |
| --- | --- |
| `duckport_scan_doab` | `https://directory.doabooks.org` |
| `duckport_scan_oapen` | `https://library.oapen.org` |
| `duckport_scan_arxiv_metadata` | `https://oaipmh.arxiv.org` |

The normal locator is `records`; an opaque continuation-token locator resumes
from a later page. `from_date`, `until_date`, `set_spec` and `record_path` are
optional text arguments. A token locator cannot be combined with date/set
selectors. `endpoint` selects an alternate service connection with its own
[HTTP secret scope](../secrets.md).

Default columns are stable OAI headers: `identifier`, `datestamp`, `status` and
`set_specs`, including deleted records. To retain provider bibliographic content,
choose an explicit XML mapping, for example:

```sql
SELECT * FROM duckport_scan_arxiv_metadata('records',
    record_path := '/oai:OAI-PMH/oai:ListRecords/oai:record', max_records := 1000);
```

`oai` is bound to `http://www.openarchives.org/OAI/2.0/`. A custom mapping infers
nested output fields and must keep the same schema across pages. This example
retains record substructure; transform it to your stable snapshot schema before
using it as [arXiv PDF metadata](arxiv.md#pdf-and-metadata-in-one-table).

## Continuation and limits

Dates/sets apply to the initial request; continuation requests send the token.
Empty pages continue if a token remains. Repeated tokens, provider errors and
schema changes fail the scan. Each execution of a prepared statement starts its
own scan; pages are fetched on demand.

`max_pages` and `max_records` are optional positive BIGINTs. `max_pages` counts
fetched pages, including the initial/resumed page and empty pages. If another
token remains at the limit, the query fails rather than presenting a truncated
harvest as complete. `max_records` deliberately caps output rows, slices the
last batch and stops without fetching another page; a current page is still
fetched and parsed in full. Omitting both limits harvests until the token ends.
