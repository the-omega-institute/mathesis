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

Both paper readers show available material groups and the repository. Published
readers, slide PDFs, interactive explanations and actual videos offer inline previews;
global language selects the reading and slide edition. Do not add defensive process
boilerplate to the materials section. No materials means no section.
Materials are exposition, never proof receipts or archive/DOI authority. The
server performs no publisher write and holds no publisher credential.

Optional `videos` and `formalization_requests` default to empty for existing
manifests and stored sidecars. Video page/MP4/poster links share the paper's
existing URL boundary; only actually built nonempty output is advertised by the
paper adapter. Requests link only to canonical positive-number issues in
`the-omega-institute/trureturing-requests`, deduplicated per paper. They are
shared work registrations, not evidence of verification, mathematical openness,
completion, dependency or newly earned credit. GitHub remains the live work
queue; issue closure does not confer a proof receipt. Public formalization
navigation, claims and the current contribution leaderboard consume this shared queue (§28–29).

Collected exposition uses the `collection` queue
with `WISHPOOL_COLLECTION_CONCURRENCY` slots, independently of external review.
New CMA collection admissions require `WISHPOOL_CMA_COLLECTION_WORKSPACE` and
an active sandbox distinct from the external review workspace's sandbox. Missing
or unavailable collection capacity leaves its original durable job waiting;
it never borrows external capacity. Saved CMA create intents retain their exact
workspace, prompts and operation identities across routing changes. The deprecated
formalization workspace key is accepted for configuration compatibility only;
it never admits automatic proof generation. Proof execution uses the shared
request workflow, not this collection lane (§28).

Administrators can inspect `/staff/processing` and the review queue's processing
panel. Mongo retains latest completion/failure per job identity, waiting reason,
retries and queue age. Live worker capacity expires after 90 seconds without a
heartbeat. This is worker admission capacity, not a promise that CMA has that
many free remote agents. Provider waits are not publication decisions. Ordinary
author and public projections do not expose these observations. Registration
and submission policy are unchanged.
