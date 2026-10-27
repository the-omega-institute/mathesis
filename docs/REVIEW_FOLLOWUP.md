# Version-bound review follow-up

An author response requests another ordinary referee round. It does not resolve an
original finding. A completed re-review now records a separate assessment of each
response admitted into that round:

- `addressed`: the reviewer considers the original concern answered on this source.
- `unresolved`: the concern remains, with an explanation of the missing repair.
- `not_checkable`: the supplied evidence or reviewer output cannot support a conclusion.

These are model review judgments, not independent execution or Lean receipts.
The original audit remains immutable. The author can read the conclusion alongside
the original finding and response, and download referenced attachments from the
exact assessed version. Conclusions from older source/statement revisions remain
visibly historical. Both JSON and Markdown dossiers include the assessments.

## Contract and validation

`RefereeAudit.response_assessments` is additive and omitted when empty. Each entry
contains `response_id`, `status`, `explanation` and `evidence: [{path, sha256}]`.
Layer 2 requires a unique assessment for every current-version/current-claims
response admitted by the round. It rejects unknown response IDs, duplicate entries,
empty or oversized explanations and attachment references that do not match the
current immutable source inventory. A model supplies paths only; the binary binds
the actual stored hashes. An unknown attachment path cannot establish `addressed`.
Missing, duplicate or malformed model conclusions become explicit `not_checkable`
entries. Extraneous model response IDs are never projected to authors.

New rounds persist `review_contract: 2`. Earlier rounds without a contract
deserialize as contract 0. Contracts 0 and 1 retain their original prompt bytes,
including when a worker resumes an admitted task after upgrading. No historical
conclusion is fabricated. Contract 1 adds follow-up instructions only to audit
tasks with actual author responses. Contract 2 additionally transports declared
attachment text and its exact stored inventory to audit, advice and letter steps
(512 KiB per UTF-8 file, 2 MiB total). Binary or larger attachments remain explicit
inventory-only entries. The inventory is recomputed and compared with the stored
source version before it is sent. A local source-directory path does not transfer
its files to remote CMA; the bounded evidence is carried in the actual prompt.
Advice and letter generation receive the sanitized audit through their existing
interfaces; neither publication policy nor proof verification changes.

## Delivery and asynchronous cleanup

An author projection hides advice and letter drafts. Their absence means the
letter has not been delivered, not that the review has finished without one.
The progress card and review panel use the same wording for this case. A private
review's headline and status badge follow the current round, so an earlier letter
cannot make a pending re-review appear ready. Finding titles and letter subjects
render inline mathematics with the same renderer as manuscript titles.

Each round now projects `progress` for referee, audit, advice, letter and formal
stages. This derives from the saved steps and contains only state, recorded
start/completion/failure times and the failure's retryability flag. It contains no
results, error text, provider handles or input digests. Hidden advice and drafts
remain hidden. This additive field is included in the author-safe JSON dossier
and described in OpenAPI; no persisted round or admitted prompt changes.

The author sees stage details and can refresh manually. Thirty-second polling
continues during work, after transient fetch failures, and between saving a
response and the worker admitting the next round. The latter is shown as
"response saved; follow-up waiting to start" rather than completed old feedback.
Fully settled rounds with a delivered letter stop background polling; focus
refresh still discovers later work. Older servers without `progress` retain the
previous polling fallback. Displayed times are recorded step times, not provider
heartbeats or estimated completion times. A failure requests attention without
promising an automatic retry or exposing private diagnostic text.

CMA acknowledges agent stopping asynchronously. Mathesis observes the agent's
actual `stopped` state before issuing a durable DELETE, avoiding an early
`agent_not_stopped` response cached under that delete key. The historical private
`stopped` progress flag records acknowledgment of the stop request; `deleted`
records observed removal (404 or a deleted state), not an asynchronous DELETE
acknowledgment. Deletion retries retain the original idempotency key. A delivered review keeps its existing queue job while
cleanup remains pending, deferring 30 seconds without rerunning the review or
spending retry attempts. The saved answer and original payer remain unchanged.
Missing agents count as already removed. Cleanup failures retain durable progress
and never substitute another payer's credentials.

On normal worker shutdown, Mathesis cancels the local polling operation and its
lease heartbeat before releasing that job back to the queue. The next worker can
resume immediately with the same admitted provider handles, without spending a
retry. If releasing the lease fails, ordinary lease expiry remains the recovery
path. A process crash still relies on that expiry; shutdown does not cancel the
remote review or create a replacement task.

## Operating the real review loop

After repairing an operational failure that left an unfinished round without a
queued job, staff can `POST /api/v1/submissions/{id}/referee/resume` with
`{"round": <current round number>}`. This requires the editor/staff role and returns
202. It requeues that round while preserving every saved step, provider task and
source identity. Existing live leases are left intact. Changed source versions or
claims, withdrawn/draft submissions and settled rounds are refused. It does not
start a new round or retry a settled failed step; use restart only when a new
review is intended and the current round has settled.

An unusable optional source attribution is omitted without erasing an otherwise
valid proof check or methodology. An audit with no usable search scope records
that limitation explicitly, allowing the existing publication policy and private
feedback to run. This does not establish literature completeness or novelty.

API-only `/readyz` checks database readiness; worker/all roles additionally check
configured executables, CMA resources and verifier readiness. `mathesis --check`
provides the same diagnostics without starting a listener. These resource checks
are not evidence that Oracle or CMA model execution works. Before a real evaluation, check the Oracle pool,
CMA workspace/profile and a task-scoped execution; follow its terminal result.
An agent creation HTTP 201 alone does not establish execution readiness.

The existing `WISHPOOL_CMA_BASE_URL` can use CMA's published direct HTTPS API with
the same NyxID user access token when direct downstream authentication is supported.
Do not automatically switch endpoints or payer after an uncertain write. Preserve
the original body, idempotency key and task handles. In the 14 October evaluation,
a JSON-media-type refusal was replayed under its original key; a separate direct
HTTP diagnostic request with its own key was admitted. This is not a reason to
rotate keys for uncertain work or to alter NyxID/CMA upstream code.

## 15 October follow-up validation

Local API tests: 272 passed, including MongoDB and TeX integration, lifecycle
projection privacy and existing version/authorization checks. Frontend: 309 tests
passed, including waiting responses before round admission, transient fetch errors,
post-audit failures, delivered-letter fencing, polling shutdown and focus refresh.
The contributor SDK's three tests passed. Formatting, Clippy, frontend lint/type
checks and production build passed. The new RUSTSEC-2026-0349 advisory required
updating rustls to 0.23.46 in both API and SDK lockfiles; cargo-deny then passed.
No advisory exception or shared CI gate was changed.

The rebuilt local API was checked against the existing real Petersen review in
`mathesis_review_loop_20261014`: three historical rounds, two delivered letters,
unchanged original audit, hash-matching attachment, safe exports, outsider 404s and
idempotent response replay. An unintercepted authenticated browser check confirmed
the new actual stage projection and its timestamps at 320 px and 1440 px, with no
overflow or browser errors. Private review omits the post-acceptance Lean probe.
These checks reused saved real Oracle/CMA results; they did not run a new paid
review or establish production deployment or independent Lean verification.
