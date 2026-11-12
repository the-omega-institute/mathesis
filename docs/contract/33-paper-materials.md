# §33. Paper materials on public pages

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

The catalog renders `site/materials-index.json` (`mathesis.materials-index.v1`)
from the snapshot, durable repository mapping and validated local manifests. Its
`generated_at` is the snapshot time; mapped papers without media retain their
repository link. Unmapped works have no index entry. The rendered hash manifest
covers the index.

Optional `WISHPOOL_MATERIALS_INDEX_URL` enables anonymous worker discovery:
HTTPS, no credentials, query, fragment or redirects, explicit Mathesis user agent,
30-second timeout and 1 MiB maximum body. Layer 2 checks the schema and RFC 3339
time, bounds the entries, and skips malformed rows individually. Every link must
stay under the same `https://papers.trureturing.org/<repo>/` or
`https://github.com/mathesis-papers/<repo>` boundary. Unknown works, private
records and mismatched public versions are skipped.

A successful index replaces a separate revision-fenced sidecar atomically, keyed
by kind, id and exact version. The last valid occurrence wins; absence removes a
previous entry. Failed transport, envelope validation or storage leaves the
previous sidecar intact. Public projections recheck live visibility and current
public version; historical and unpublished versions cannot borrow links. Delivery
digests, receipts, statements, review work and contribution credit never change.

Both paper readers show only available material groups and the repository, with:
“Prepared by Mathesis from the manuscript; explanations and illustrations are not
verification.” The Chinese copy lives in `zh.ts`. No materials means no section.
Materials are exposition, never proof receipts or archive/DOI authority. The
server performs no publisher write and holds no publisher credential.
