# Knowledge index admission for the chrono-cd deployment, 24 October 2026

This is the operator acquisition record and deployment runbook for the first
Trureturing publication proposed for the live Mathesis deployment at
<https://mathesis-omega-research.apps.chrono-ai.fun> (chrono-cd project `omega`,
app `mathesis`). It follows [KNOWLEDGE_OPERATIONS.md](KNOWLEDGE_OPERATIONS.md).
The caller supplied the independent observations and scratch-MongoDB dry-run
receipt below. This record prepares the production operation; it does not claim
that the owner has signed, an image has been published, or production has been
written. Without a configured admitted index, contract-3 publication rounds
retain an incomplete knowledge gate.

## Publication identity and independent evidence

| Coordinate | Recorded value |
|---|---|
| Trureturing source commit | `e7dcc89974fd44cabd0f10781eccc135eddf8d1c` (merge of upstream PR #17344, 2026-10-15T10:23:41Z) |
| Source tree | `442f387c1013f60d23f70faa1927ea29fda57d21` |
| Bundle identity | `sha256:` plus the Projection `SHA256SUMS` digest in the byte table below |
| Pages release | `the-omega-institute/trureturing-pages`, tag `pages-source-e7dcc89974fd44cabd0f10781eccc135eddf8d1c`, published 2026-10-15T12:49:05Z |
| Source CI | [38719956711](https://github.com/the-omega-institute/trureturing/actions/runs/38719956711), `CI`, push to `dev`, attempt 1, completed/success at the exact source SHA |
| Report producer CI | [38727419398](https://github.com/the-omega-institute/trureturing/actions/runs/38727419398), `CI publication verification`, push to `integration-ci-fib-isolation-tests`, attempt 1, completed/success at the exact source SHA |
| Original report release | `the-omega-institute/trureturing`, tag `lean-cache-verify-v1-db584cd6d46c92f209a44c0f1c829460d327499d-linux-arm64-report-stratalint-raw-lean-report-v3-env-43c54620c14f-ci-38727419398-1` |
| Pinned running API image | `ghcr.io/the-omega-institute/mathesis-api@sha256:c9f9b38c03326ffc445bbb7967be9d2c31161ab6c689e5e595c364a047389835` |
| Report-referenced source files | `9783` |
| Source license | Original Apache-2.0 `LICENSE`, SHA-256 in the byte table below |

On 24 October the caller re-observed the commit/tree and both successful runs
through the GitHub REST API. The source was an ancestor of `dev` (`compare
source...dev`: `ahead`, `behind_by 0`). The Pages projection and original report
were independently downloaded and hashed. All 9,783 report-referenced source
files matched their individual `source_sha256` values locally. These observations
agree with [the 16 October consumption record](KNOWLEDGE_CONSUMPTION_VALIDATION_2026-10-16.md).
The implementation worker also read the published report `manifest.json` and
confirmed the five-part inventory below. Manifest trust booleans, branch names
and the presence of CI URL strings are not authentication; the operator's
independent observation is the admission evidence.

## Byte identities checked before build

The coordinate and byte tables in this record are the authoritative verification
values. Before any download, the workflow parses them, rejects malformed or
duplicate rows, checks the report part sizes sum to the recorded archive size,
and compares the exact checked-in admission JSON bytes to its table digest.
It derives the workflow environment, checksum lists and part inventory from those
rows; the workflow carries no second set of digest pins. The admission's required
identity/license/CI fields must match the same record before downloads. A changed
record digest with unchanged admission bytes fails this gate. The receipt retains
the exact record used by the build.

All SHA-256 values below are lowercase hexadecimal. The workflow checks the
bundle checksum, runs `sha256sum --strict -c SHA256SUMS`, checks both envelope
files separately, and permits exactly these eight regular projection files:
`SHA256SUMS`, `release-manifest.v1.json`, `truth-release-publication.v1.json`,
`raw-lean-report.json`, `report-provenance.v1.json`, `source-snapshot.v1.json`,
`truth-export.v1.json`, and `truth-graph.v1.json`. The five content artifacts are
bound by the pinned `SHA256SUMS` bytes; the two envelope files have independent
pins because the bundle checksum alone does not cover their bytes.

| Input | SHA-256 |
|---|---|
| Projection `SHA256SUMS` (bundle digest preimage) | `6c92665f572be41df6f8fad0d246f4e141e9cf036196608239792f787121ec26` |
| Projection `release-manifest.v1.json` | `5790d73ce903d352f1c149aa4741cd5676f189a4aa6e1bc20acb2d577b707306` |
| Projection `truth-release-publication.v1.json` | `db5b1c45252e336769a61574017542ad5d914df360453b8e9fa1f799bfbd4e01` |
| Complete concatenated report archive (7,556,369,620 bytes) | `e8b26b3e03f61cc5f068c1f21648cd2ff8f683836b51f6f7d4c17aa24fd847dd` |
| Source `LICENSE` | `45404ee75576b053a86e89e545fd6987a607107cf4d846fa78dcb6a233d53447` |
| Exact checked-in admission JSON, including trailing newline | `cb29f4b4abcea38c575772b3d89281e85c3440d46d4afde0cffc97ed16147838` |

The report release's `manifest.json` must contain exactly this ordered part
inventory, the whole-archive hash above, the recorded source commit, producer
run and attempt. Each part is hashed and size-checked against both this inventory
and the manifest while streaming. The complete compressed stream is hashed and
drained to EOF even if tar reaches its end marker earlier. The runner never
stores the 7.56 GB archive or its parts.
Manifest comparisons use the required part fields and accept integer counts/run identifiers as JSON integers or decimal strings; every name, order, digest and byte count must still match.

| Part | Bytes | SHA-256 |
|---|---:|---|
| `lean-build.tgz.part-00` | 1,610,612,736 | `47df5bf4a536eb19a7c1dffb21dc353e8c4ff8849fcb9057513b47dcf4195c47` |
| `lean-build.tgz.part-01` | 1,610,612,736 | `34899a487f4c86220cb8f83fb7152c1809893b299f7b90c3b3f004f88c34194a` |
| `lean-build.tgz.part-02` | 1,610,612,736 | `a888a8d295ef3abe866942bd2d7f8127a33a785d47ab92550180a201001e1308` |
| `lean-build.tgz.part-03` | 1,610,612,736 | `ebc7d44662edafb84d3e0ae2a4a8360724b362b46f27dcdf4d62f67ba584f195` |
| `lean-build.tgz.part-04` | 1,113,918,676 | `3a1b2008d4834f94ab4cdddedc8e46e06ca5197f8d3d7817e4d3513e6acc5d43` |

Only the following five regular members are copied out of the report archive.
The expected `build/stratalint/` prefix comes from the caller's local extraction;
the workflow fails if any exact member is absent, duplicated or not regular.
No archive member is executed and the materials ZIP is never extracted.

| Member under `build/stratalint/` | SHA-256 |
|---|---|
| `raw-lean-report.json` | `62f6bb798078ea4789d289aeaa0c89f98f685b7df85a0bd90fe9702a1255feb5` |
| `raw-lean-report.json.input.attestation` | `f66bedf142f54faadf80faf9826600d92c3c15fc3037d0fff7859c246834756a` |
| `raw-lean-report.json.materials.zip` | `8d42ab8216ee30ebfd4ee59f190409161104f7ed8905ed9c40e3a42bfa7db478` |
| `raw-lean-report.json.provenance.json` | `28d30d49f0f5cbedc1e2bce6c31d72f9e458f8399723af594af31895adff10cb` |
| `raw-lean-report.json.sha256` | `64039b5d517600b5f056c690dcfc1d3f8fe251c1a77ca1fd50c664f2fb4ab82d` |

The projection and report raw JSON must compare byte for byte before becoming
hardlinks in the scratch build context. For **every** entry of the raw report's
top-level `modules` list, the workflow hashes the source tarball's regular
`source_path` and checks `sha256:<hex>` against `source_sha256`. It requires 9,783
unique safe module paths and copies only those paths plus `LICENSE` into the
image. The 149 unreferenced `.lean` files among the caller's original 9,932 files
(including `tools/lean-inspector/*`) are excluded. The source tarball is consumed
as inert data at the exact source commit; no upstream Lean, Lake or script runs.

## Operator admission boundary

[KNOWLEDGE_ADMISSION_2026-10-24.json](KNOWLEDGE_ADMISSION_2026-10-24.json) contains
exactly the fields required by `KNOWLEDGE_OPERATIONS.md`: `identity`,
`admitted_by`, `source_ci_url`, `report_ci_url`, `protected_source_evidence`, and
`license_sha256`; `identity` contains only `bundle_digest`, `source_commit`, and
`source_tree`. The signer is the owner's decision. No implementation worker may
supply an operator attestation on the owner's behalf.

The caller will replace `ADMITTED_BY_PLACEHOLDER — owner decision pending (see record)` only after the owner answers, then calculate its SHA-256 (including the trailing newline) and update **only the `Exact checked-in admission JSON, including trailing newline` row in the byte table above**. This is the one admission-digest edit; the workflow reads `ADMISSION_SHA256` from that row and needs no edit.

The workflow checks that digest, the exact field inventory, all three identity
values, both exact CI URLs and the license digest. It rejects empty evidence and
any signer still containing the pending marker before generating or building
the carrier. The admission digest binds the entire evidence text as well.

## Vehicle

chrono-cd provides services with one `health_path`, and its MongoDB add-on is
reachable only inside the namespace. There is no exec, job or command override.
[The one-off workflow](../.github/workflows/knowledge-import-once-20261024.yml)
therefore generates its Dockerfile and POSIX entrypoint in the runner's scratch
build context. There is no carrier directory or tracked addition under `infra/`.
The generated entrypoint must pass `sh -n` and `shellcheck --shell=sh` on the `ubuntu-24.04` runner; ShellCheck availability there is ASSUMED-UNVERIFIED, and a missing binary fails the step.

The base is the running importer/reader image in the coordinate table above.
The capability test and Docker build both use that one recorded base digest.
It is Debian trixie-slim with TeX Live. Perl is expected through TeX Live's
dependencies; its presence and both `IO::Socket::INET` and core `JSON::PP` capabilities remain a build-time
gate, not an assumption accepted as proof. Before downloading inputs, the
workflow runs the exact probe listener on that base as uid 65532 with networking
disabled, verifies GET and HEAD `/healthz` return 200 while GET `/other` and POST `/healthz` return 404, and checks
the importer binary and required shell utilities. Missing capability fails
closed; no packages are installed.

The carrier runs **no Mathesis serving process**. A probe-only Perl HTTP
listener binds `0.0.0.0:8080`, answers `/healthz` and stays alive for log collection.
Only GET and HEAD on the exact `/healthz` path succeed; every other method/path returns 404, and HEAD responses omit the body.
The POSIX entrypoint invokes
`mathesis --knowledge-import /opt/knowledge/projection /opt/knowledge/report /opt/knowledge/sources /opt/knowledge/admission.json`
exactly once per container start, as uid 65532. It logs `exit_status`, and on
success `knowledge-import: ok` plus only the top-level `id`, `candidate_modules`,
`indexed_modules`, `omitted_modules`, and `admitted_at` fields. Core Perl `JSON::PP` structurally selects those five scalar top-level fields,
excluding nested blob `id` fields and accepting any JSON whitespace or key order.
Failure logs retain the exit status and last 40 stderr lines when available.
Header parsing, header display and stderr diagnostics are guarded: absent files
or unwritable `/tmp` report failure without terminating the probe. Health means the
probe is alive; importer success must be confirmed separately in logs. The
listener stays alive on import failure to avoid an automatic retry loop, and
termination signals stop both child processes. No runtime input fetch occurs;
MongoDB access uses only the platform add-on.

The image contains the pinned base, verified inputs, exact admission JSON and
entrypoint. Modes are set before the single payload COPY to avoid duplicating a
large layer with a recursive chmod. The default Docker builder uses its local
image store for the build, real-entrypoint smoke test and push; there is no
second image archive or local export. The smoke test runs without MongoDB or
secrets, confirms the real binary's expected memory-storage rejection and exit
status, verifies uid 65532 and `/healthz` before and after the failure, and checks
that `/knowledge/capabilities` is not served by the probe.

The successful workflow publishes artifact
`mathesis-knowledge-import-20261024-receipt`, including `image.digest` (the
immutable RepoDigest), `receipt.json` (base digest, workflow commit/run, admission
digest and source identity), exact record/admission/entrypoint/Dockerfile/probe files,
and base/smoke logs. Deployment must cite `image.digest`, never the mutable push
tag. Runtime gates and image publication are caller-owned and have not run in
this implementation worker.

## Local dry run

The caller supplied this completed receipt on 24 October: a debug build of the
`claude/arxiv-experience` worktree, with importer sources identical to `develop`,
imported the inputs above into scratch MongoDB. This is evidence for the index
content and importer behavior; it is not an execution of the pinned Linux
carrier against production.

| Measurement | Result |
|---|---|
| Snapshot `id` | `sha256:2838a557bfddda5a1f9410e0dcf596a67112c0ec0f163c4cf1a787abcce15d2b` |
| `candidate_modules` | 8,679 |
| `indexed_modules` | 8,673 |
| `omitted_modules` | 6 |
| Retained original/transport/license artifacts | 13 |
| MongoDB `dataSize` | 1,256,092,211 bytes |
| Wall time | 4 min 27 s |

The same revision was recorded on 16 October. The six omissions remain explicit
source-size omissions; the 9,783 checked source paths are a larger inventory
than the 8,679 declaration-bearing candidates. The live import must reproduce
the exact revision above. Admission text and admission time do not change it.

## Idempotency, failure and recovery

These references were read from the unchanged implementation in this worktree:

| Behavior | Code evidence and operational meaning |
|---|---|
| Header inserted last | [knowledge.rs:219](../api/crates/mathesis/src/knowledge.rs#L219) uploads the license/artifacts and index before constructing the header and calling `insert_knowledge_snapshot` at line 246. A failure before that call leaves no newly inserted header; completed blobs may remain. |
| Same inputs rerun | [knowledge.rs:195](../api/crates/mathesis/src/knowledge.rs#L195) loads/validates the existing index, compares the 13-entry artifact inventory and hashes, then streams verification of every retained blob before returning the prior header at line 217. [store/knowledge.rs:35](../api/crates/mathesis/src/store/knowledge.rs#L35) checks actual blob bytes and lengths. |
| Same revision, changed artifacts | The inventory comparison at [knowledge.rs:206](../api/crates/mathesis/src/knowledge.rs#L206) fails. A concurrent duplicate insert also compares publication, index and artifact hashes/lengths at [store/knowledge.rs:127](../api/crates/mathesis/src/store/knowledge.rs#L127) and rejects conflicting immutable contents. |
| Revision identity | [index.rs:115](../api/crates/layer3-review/src/knowledge/index.rs#L115) hashes protocol, publication, candidate count, checked source/declaration modules and omissions. Changed index content/publication/omissions yields a different revision and leaves the old snapshot untouched. Different artifact or admission bytes alone do not necessarily yield a different revision. |
| Admission text/time | [knowledge.rs:235](../api/crates/mathesis/src/knowledge.rs#L235) adds `admitted_at` and admission after index construction. These are outside the revision preimage, so an identical rerun returns the original admission/time instead of replacing them. |
| Uncertain final insert | [store/knowledge.rs:104](../api/crates/mathesis/src/store/knowledge.rs#L104) performs the final insert and handles duplicate identity. Its line 143 comment explicitly retains blobs after uncertain writes. If the final insert result is lost or errors ambiguously, restart the container with the same carrier digest/admitted inputs using the mechanism below and validate the stored result; never assume the header is absent or delete blobs eagerly. |
| Orphan handling | Each retry without a stored header repeats uploads of approximately 1.26 GB (if uploads complete); any prior completed uploads remain orphan blobs. A confirmed existing header takes the validation path and avoids these uploads. Failed or duplicate uploads can leave unreferenced blobs. The existing [offline storage audit and collection procedure](OPERATIONS.md#review-history-and-storage-recovery), lines 104–133, audits read-only, retains young/unknown-age objects and requires a verified backup and all writers stopped before `--storage-gc --offline`. Removing this service does not delete imported data or authorize live collection. |
| CLI output | [main.rs:38](../api/crates/mathesis/src/main.rs#L38), lines 38–44, selects the Mongo import command; [main.rs:36](../api/crates/mathesis/src/main.rs#L36) calls `Config::from_env` first. Line 66 emits serde_json pretty JSON and exits. This command does not start application serving or worker composition. |

## Caller runbook for the live import

The current platform observation was generation 14 / revision 17 with four
services: `verifier`, `web`, `wishpool-api`, and `worker`. Read the current values
again before any whole-spec apply. Use the observed generation as `If-Match`;
on conflict, re-read and preserve intervening changes. Save the pre-change spec,
generation, each resulting generation, workflow receipt and complete import logs
with the operator deployment record. Do not record credentials or secret values.
The production write and operator signature remain owner/caller decisions.

| Step | Exact chrono-cd spec change | Evidence that confirms it |
|---|---|---|
| 1. Preserve state and approve inputs | No spec mutation. Read/save current spec and generation; confirm the four service names and absence of `WISHPOOL_KNOWLEDGE_INDEX`. Resolve the owner signer, update JSON and the single admission-digest row in this record, then let the one-off workflow complete. | Retained spec/generation and owner decision; successful workflow checks and receipt artifact containing the exact `image.digest`, workflow commit/run and admission digest. An unsigned draft or absent image receipt stops the operation. |
| 2. Add the carrier | Whole-spec apply with `If-Match`: add `services.knowledge-import`, `image` = exact RepoDigest from `image.digest`, `replicas: 1`, `port: 8080`, `health_path: /healthz`; no volumes, `secret_keys` or `secret_files`. Set only ordinary env `WISHPOOL_BIND=127.0.0.1:8080`, `WISHPOOL_PUBLIC_URL=https://mathesis-omega-research.apps.chrono-ai.fun`, `WISHPOOL_STORAGE=mongo`, `WISHPOOL_AUTH_MODE=dev`, `WISHPOOL_ROLE=api`. Copy the `wishpool-api` MongoDB `addon_env` mapping verbatim: `mongodb {uri: WISHPOOL_MONGODB_URI, database: WISHPOOL_MONGODB_DATABASE}`. Leave `egress` at the platform default; omit `WISHPOOL_KNOWLEDGE_INDEX`. Preserve the other four services and add-ons. | Apply returns the next generation; status lists a fifth service using the receipt's digest, one replica, and a passing `/healthz` probe. Logs show `knowledge-import: starting`. No existing API/CMA/NyxID secrets are copied. |
| 3. Confirm the immutable header | No spec mutation. Read `knowledge-import` service logs until `exit_status=0`, `knowledge-import: ok` and all five top-level header fields appear. Do not infer import success from health. | Retain complete logs: `id` exactly `sha256:2838a557bfddda5a1f9410e0dcf596a67112c0ec0f163c4cf1a787abcce15d2b`, counts 8679 / 8673 / 6, and the actual `admitted_at`. **STOP if the id differs from the dry-run revision**, the header is incomplete or the importer reports failure. Do not enable readers. For an uncertain final insert, restart using the same carrier digest/inputs as described below and validate the prior header. A divergent-id STOP occurs after that complete snapshot already exists; retain it as immutable data and never configure it on readers. |
| 4. Remove the temporary service | Re-read spec/generation; whole-spec apply removes only `services.knowledge-import`. | Status lists exactly the original four services; the carrier is gone. Snapshot and referenced immutable blobs remain by design. Save the confirmed header and removal generation in the deployment record. |
| 5. Enable both readers | Re-read spec/generation; set `WISHPOOL_KNOWLEDGE_INDEX=sha256:2838a557bfddda5a1f9410e0dcf596a67112c0ec0f163c4cf1a787abcce15d2b` in **both** `wishpool-api.env` and `worker.env`. Preserve all other values. | Both services complete rollout; their saved spec pin equals the confirmed header. GET `/knowledge/capabilities` on the API succeeds and reports the source query/ranking protocols and coverage limits; retain the response. This endpoint describes protocols and does not attest snapshot availability. Step 6's durable round pin/query receipts confirm consumption. |
| 6. Test publication (K7) | No further spec mutation. Run one publication-mode manuscript round, with confirmed claims, on the live deployment. | Retain manuscript/version/round identifiers and review dossier or `/review.md`: knowledge contract/gate state, pinned snapshot, required raw/reformulated query receipts, comparisons, and decision or explicit incomplete-gate reason. The author-facing submission page shows the decision or knowledge-gate limitation in plain words. A `review_only` round does not satisfy this step. |
| 7. Roll back reader configuration if needed | Re-read spec/generation; remove the two `WISHPOOL_KNOWLEDGE_INDEX` env values from `wishpool-api` and `worker`; finish both rollouts. If the carrier still exists after a failed attempt, remove that temporary service as well. | Both services roll out and the saved spec confirms both pins are absent. New publication rounds expose incomplete knowledge. Historical rounds retain their original pins/receipts. Immutable imported data remains by design; rollback performs no MongoDB deletion. |

To retry, re-read the current spec/generation and use whole-spec applies with
`If-Match` to set only `services.knowledge-import.replicas: 0`, confirm zero live
carrier replicas, then set it back to `1`. Alternatively, remove the temporary
service, confirm it is gone, then re-add the identical service with the same
receipt RepoDigest and admitted inputs. Preserve the other services/add-ons at
every apply and save each generation and the new start's logs. There is no exec
primitive or retry endpoint: the entrypoint invokes the importer only once per
container start. The healthy probe does not automatically rerun a failed import.
If no header was stored, each fully completed retry re-uploads approximately
1.26 GB; earlier completed blobs become orphans and are collected **only** by the
existing offline storage audit/collection procedure cited above. If the uncertain
insert did store a header, the rerun validates and returns it instead of uploading
again ([knowledge.rs:195](../api/crates/mathesis/src/knowledge.rs#L195)).

A differing revision is detected only from the completed import's log: the header
has already been inserted at [knowledge.rs:246](../api/crates/mathesis/src/knowledge.rs#L246).
That divergent snapshot remains immutable and content-addressed, allowed by the
production write policy (constraint 4). Remove the carrier and retain the evidence;
**never configure the divergent revision on readers**. Stopping at this point does
not undo the snapshot, and removing a service performs no storage collection.

Peak importer memory on chrono-cd is **unmeasured; memory headroom is unverified**.
The importer holds the approximately 248 MB raw report and other projection bytes
in memory ([knowledge.rs:143](../api/crates/mathesis/src/knowledge.rs#L143)), builds
the source index, then serializes that index into another buffer
([store/knowledge.rs:243](../api/crates/mathesis/src/store/knowledge.rs#L243)).
Inspect service logs and platform status/telemetry on failure: `exit_status=137`
means SIGKILL (128 + 9), likely OOM; corroborate with platform OOM/memory evidence.
`exit_status=143` means SIGTERM (128 + 15), including platform/service termination.
If the entire container is killed, the entrypoint may emit no exit-status line;
use the platform's container exit status and telemetry. Save the evidence before
an intentional restart; neither signal confirms that a header is absent.

`WISHPOOL_BIND` is loopback only for the importer's existing configuration
validation; the separate probe always binds port 8080 on all interfaces. [main.rs:36](../api/crates/mathesis/src/main.rs#L36) calls
`Config::from_env` before command selection at lines 38–44, so the runbook env must
satisfy [config.rs:228](../api/crates/mathesis/src/config.rs#L228) (bind/origin),
[config.rs:238](../api/crates/mathesis/src/config.rs#L238) (MongoDB), and
[config.rs:258](../api/crates/mathesis/src/config.rs#L258) (auth and loopback rule).
The import CLI then takes its early path before any API or worker role is composed, so
dev auth introduces no sign-in endpoint. This minimal existing configuration
avoids requiring the production API's NyxID/CMA secrets. Both reader roles need
the index value because normal composition constructs the knowledge source for
any role. No new application config key is introduced.

The probe requires no egress. Whether platform `egress: none` still permits the
MongoDB add-on has not been verified, so this one-off service uses the platform
default rather than asserting that policy is safe. The only runtime dependency
needed by the import is MongoDB through `addon_env`.

## Execution status and remaining caller evidence

The local dry run above is complete. The owner signer decision, one-off Linux
base/listener proof and image smoke test, published RepoDigest receipt, live
MongoDB import, spec generations/logs, reader rollout/capabilities response and
K7 publication-mode manuscript acceptance are still caller-owned. After those
operations, append the actual receipt and observation references to this record;
do not substitute a tag for the image digest or treat this prepared runbook as
a production receipt.
