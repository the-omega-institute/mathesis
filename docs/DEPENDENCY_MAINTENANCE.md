# Timestamp dependency maintenance

The dependency audit added `RUSTSEC-2026-0543` on 17 November 2026. It marks all
Chrono versions unmaintained, lists no patched version, and reports no specific
security vulnerability. Mathesis already uses Chrono 0.4 timestamps throughout
its domain, serialized API, durable records and MongoDB BSON date adapter.

The exact informational advisory is accepted in `api/deny.toml` while that
existing representation is maintained. This affects the API workspace audit;
it does not skip dependency checks, allow other advisories, change CI workflows
or alter persisted timestamps. New Chrono security advisories remain errors.

A future replacement must preserve RFC 3339 API bytes, BSON instants, expiry and
lease comparisons, calendar behavior and historical receipt identities. Inspect
MongoDB's supported date adapters and transitive dependencies before choosing
`jiff` or `time`; replacing only the direct import may leave Chrono in the graph.
Remove the informational exception once the compatible migration is complete.

The same audit found `RUSTSEC-2026-0548` in ZIP 9.0.3. The lockfile now selects
the patched ZIP 9.0.5. Mathesis reads archives through `ZipArchive` and handles
the inspected entries directly; it does not validate central-directory names
and then extract with `ZipStreamReader` or `read_zipfile_from_stream`.
