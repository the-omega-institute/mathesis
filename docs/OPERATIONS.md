# Operations and recovery

## Local use and CMA identity

The application runs locally with memory storage and development sign-in as
shown in the README. Use MongoDB when work needs to survive restart. TeX Live
must be available to compile manuscripts. Reviews require both Oracle and an
auditor; Lean work needs a prepared matching workspace or verifier deployment.

NyxID CLI mode uses the operator's local login to access a remote CMA workspace.
It does not register Mathesis as a permanent service inside CMA. Mathesis
creates temporary agents for individual steps and retains their identities for
recovery and cleanup. A session expiry needs a new `nyxid login`. Production
uses HTTP through the NyxID proxy and a dedicated operator API key scoped to CMA,
or the explicitly connected author's
ordinary NyxID credential, with the existing durable payer selection.
Keep the operator key in `WISHPOOL_CMA_TOKEN_FILE`; a copied local access token
expires without renewal. Key acceptance through the proxy does not imply that
direct CMA origins accept the same key. Verify the deployed route with resource
reads, then a complete Agent response before declaring review execution ready.

Current CMA execution uses `WISHPOOL_CMA_AGENT_PROFILE` with the explicit inline
profile `{"model":"gpt-6.1-sol","reasoning_effort":"high","permission_mode":"autonomous"}`.
New tasks use `POST /api/v1/agents` with the owned workspace and this exact profile;
the durable lifecycle receipt is saved and polled until it yields an Agent.
Legacy `{id,revision}` configuration and already saved creation intents retain
their original route and bytes for historical servers and recovery. Do not send
a profile ID to the current lifecycle endpoint. BYOK also requires the author's
owned, ready workspace in an active sandbox using `own_credentials`.

For company-managed execution, use an operator workspace whose sandbox profile
selects `chronoai_managed` and leave `WISHPOOL_CMA_BYOK_ENABLED=false`. A personal
NyxID sign-in identifies the person; it does not by itself mean a personal Codex
subscription supplies the model. Model access, resource ownership and usage/quota
attribution are separate facts. Confirm the company's actual resources and
quota configuration before claiming the complete review service is available.

Current CMA cleanup first requests lifecycle DELETE and waits for observed
deletion. If the deployment explicitly requires a stopped Agent, it persists that
prerequisite, calls the documented `/stop`, and waits for a terminal read before
deleting with a distinct post-stop key; the earlier rejection can be cached.
Legacy CMA cleanup
retains each Agent until deletion is observed. A fresh `stopped`
or `failed` Agent state permits the existing deletion request; a stop acknowledgement
alone does not. Failed startup Agents can refuse stop with `agent_not_ready` but
remain eligible for deletion. Cleanup preserves their recorded failure and task
identity, and never interprets cleanup as a successful review or a new attempt.

## Readiness and admission

Deploy the API and workers from the same revision when introducing review
contract 3. Gracefully stop old workers before admitting new rounds; preserve
their durable remote task identities so contracts 0–2 resume with their existing
prompts. New binaries read historical records, but old binaries do not understand
`undetermined` proof shapes or the new rejection variants. A rollback needs a
reader compatible with contract 3 and the GridFS history format; do not rewrite
or delete settled rounds to make an older binary load them. See
[REVIEW_METHODOLOGY.md](REVIEW_METHODOLOGY.md).

Run `cargo run -p mathesis -- --check` from `api/`, with the same environment as
the intended process. It prints JSON to stdout, logs to stderr, and exits nonzero
if configured dependencies are unavailable. Composition may initialize service
accounts; the command creates no CMA agent and requests no model execution.

- `/healthz` is process liveness. It does not contact providers.
- `/readyz` checks MongoDB. Worker/all roles also check the TeX executable,
  CMA workspace/sandbox resource state and the configured verifier. Local Codex
  and Oracle CLI executables are checked when configured.
- API-only readiness does not depend on CMA, so a provider outage does not remove
  access to existing papers and reports. Check the worker endpoint or use
  `WISHPOOL_ROLE=worker` for execution diagnostics.
- Runtime observations are cached for 30 seconds. CMA checks have an eight-second
  deadline; remote verifier checks have five seconds. Probe timeout is 20 seconds.
- `disabled` means not configured; it is not the same as `unavailable`.
  `provider_execution_verified` is always false: these checks cannot prove
  model credentials, profile readability, quota, Oracle pool availability or
  mathematical quality. `automatic_review` describes the basic execution
  prerequisites, not a completed end-to-end test.

Before a new referee request starts its first Oracle call, the worker checks
the eligible CMA account's resources with a 15-second overall deadline. Failure
records an actionable retryable step and avoids starting that new Oracle task.
After fixing configuration, restart the settled review through the existing
author/staff workflow. A persisted request identity bypasses this new admission
check, preserving replay keys and the existing payer contract. Read-only
preflight itself never admits a payer or creates an agent.

## Worker capacity

Keep one worker replica for the TeX cache volume. Each process runs one compile
lane, one verification lane and `WISHPOOL_REVIEW_CONCURRENCY` remote review lanes
(default two, range one to eight). Every job has its own lease, heartbeat,
cancellation and revision fences. Compilation skips remote agent cleanup.
Long CMA work cannot hold the compile or verification lane. This is bounded
concurrency, not automatic scaling; model quotas, CPU and memory still constrain
throughput. Start with the default and measure provider failures and job latency.

The internal collection lane reports zero capacity and leaves its durable jobs
unclaimed when `WISHPOOL_CMA_COLLECTION_WORKSPACE` has no configured advisor.
This does not consume retries or repeatedly overwrite the same provider error.
Ordinary review, compilation, verification and capacity observations continue.
After configuring a valid internal workspace, restart the worker to resume the
same queued paper/version jobs. This does not provision a sandbox or change the
separate catalog material-production queue or shared formalization requests.

## Fixed Mathesis sandbox responsibilities

The operator allocation is two sandboxes, with one fixed purpose per sandbox.
Independent jobs run concurrently inside their assigned sandbox; one admitted
job retains its original workspace and provider identity across restarts.

| Responsibility | Sandbox | Workspace | Concurrent model jobs |
| --- | --- | --- | --- |
| Internal collected-paper materials, through `mathesis-papers/catalog` | `sbx_f62ac11bfe7c4fffb57921ade11b5271` | `wks_194616ce9af14633981515b6f557578c` | At most 6 authoring agents |
| External submission review and further research | `sbx_d13f0b6ba767486ca93474b5c84af6de` | `wks_f5b364b780ab47608282887d2a4b2399` | At most 6 review jobs |

Both allocations report 2000 CPU millis, 8192 MiB memory and 64 GiB disk.
The internal setup check read the actual 8 GiB cgroup memory limit; its idle
memory use was approximately 367 MiB. Six jobs follows Outreach's operating
limit for the same allocation. This is a memory budget selected by the
operator, not an automatic memory controller or a six-job load-test result.
Heavy computational work needs a lower per-sandbox limit.

Catalog's existing `authoring_concurrency: 6` is the sole internal material
production scheduler. Its `CMA_COLLECTION_WORKSPACE` selects the internal
workspace above; `CMA_EXTERNAL_WORKSPACE` identifies the review workspace.
The platform worker's `WISHPOOL_REVIEW_CONCURRENCY` is six. Keep
`WISHPOOL_CMA_COLLECTION_WORKSPACE` unset while catalog owns internal stories
and materials, so the legacy exposition queue cannot launch a second copy.
Catalog materials are consumed through the existing versioned material index.
Saved older catalog operations retain their original workspace until their
work and checked cleanup finish; changing the assignment affects new jobs.
Formalization execution remains in the shared request workflow and consumes
no additional dedicated Mathesis sandbox.

The internal sandbox was created through CMA's authenticated public lifecycle
API. The NyxID proxy's new-sandbox route still returns
`cma_sandbox_account_slot_occupied` for the held historical reservation tracked
in ChronoAIProject/NyxID#2590; using the public CMA API did not alter or release
that reservation. Existing workspace observations and execution use their
ordinary supported interfaces.

## Review history and storage recovery

Referee aggregates larger than 512 KiB of JSON are saved as complete GridFS
snapshots. MongoDB retains a small header with format, revision and blob digest.
Reads verify size, checksum, identity and revision. Inline legacy documents
remain readable and migrate on their next large write. A failed revision compare
discards only its own definitively unreferenced new snapshot. Ambiguous database
write outcomes retain data for recovery.

The snapshot is the full aggregate, so frequent updates to a long review add
storage and I/O costs. Replaced snapshots remain available until offline
collection to avoid deleting data under concurrent readers. Monitor GridFS
growth and plan maintenance; this is not an incremental per-round event store.

Back up the complete database, including `papers.files` and `papers.chunks`, as
one consistent unit. Test restoration before upgrading production. Older server
versions do not understand the new snapshot header: a binary-only downgrade is
not supported after migration. Restore a compatible backup or keep the new
reader during rollback.

With the normal MongoDB configuration loaded:

```sh
mathesis --storage-audit
```

The audit reports unreferenced completed files older than seven days, missing
referenced files, and aged chunks left by interrupted uploads. Chunk age must
be established for the whole file group from ObjectIds; unknown or recent ages
are retained. The audit is read-only. Under live writes its result can be a
transient observation, so repeat offline before acting.

To reclaim storage:

1. Take and verify a consistent backup.
2. Stop **all API and worker instances**, and prevent automatic restart.
3. Let active leases expire (or finish graceful shutdown), then run the audit.
4. Resolve missing referenced files before any collection.
5. Run `mathesis --storage-gc --offline`, inspect its JSON report, then restart.

The audit and collector also read and verify every GridFS review snapshot using
the normal reader, including its checksum, format, submission identity and revision.
An unreadable history aborts collection before any deletion; restore that history
first. References inside a decoded snapshot are retained alongside its header.

The collector refuses active job leases and missing referenced files. `--offline`
is the operator's assertion that every writer is stopped; it is not a distributed
maintenance lock and cannot detect a concurrent upload from a live API. Never
schedule it against a running deployment. The seven-day minimum is a safety
margin, not a substitute for stopping writers.

## Prepared Lean verifier

The verifier runs without credentials and does not install submitted dependencies.
The checked-in reference base pins stable Lean 4.35.0 and matching Mathlib
`56a005855ea17e0b0958f73ec801ae140b0a1679`. For Linux amd64:

```sh
docker build --platform linux/amd64 -f infra/verifier/Lean.Dockerfile -t mathesis-lean:local .
docker build --build-arg LEAN_IMAGE=mathesis-lean:local \
  -f infra/verifier/Dockerfile -t mathesis-verifier:local .
```

The second build runs the real Lean proof, disproof, wrong-type, forbidden-axiom
and multiple-module cases against the base. Production should publish the
prepared base and pass its immutable registry digest as `LEAN_IMAGE`; the local
tag is for build validation. Preserve old pins when historical verification
bundles must remain reproducible. Download/build requires network access and
substantial disk space; runtime verification has no egress.

`/readyz` checks the pinned manifest, installed Lean executable and built Mathlib
artifact. `/environment` returns those resolved pins only after that readiness
check succeeds. Historical Mathesis proof checks retain their original pins;
new formalization execution uses shared Trureturing requests (§28–29). Keep the
previous service available through `WISHPOOL_VERIFIER_LEGACY_URL` before changing
the primary verifier; historical targets and active conversations keep their
original environment. `/healthz` only checks that the process is serving. Ordinary Rust tests
explicitly mark real Lean cases ignored; selecting them without
`WISHPOOL_TEST_LEAN_WORKSPACE` fails instead of silently passing. CI builds the
verifier image and validates its Kubernetes manifests alongside API/web/worker.

A workspace may declare further prepared libraries in `mathesis-libraries.json`;
readiness then also requires their built artifacts and `/environment` lists them.
The reference image declares `trureturing` with root `D5`, compiled from source
at the revision pinned in `infra/verifier/lakefile.toml` for the modules of
`library-modules.txt` and their imports. Deploy the API and worker before a
verifier that lists libraries. See [LIBRARY_BINDING.md](LIBRARY_BINDING.md).

## Scope that still needs product or infrastructure work

- No automatic provisioning of a CMA workspace/profile/model connection is
  implemented. An operator or author prepares those resources.
- CMA quota attribution is retained, but metered donation totals require a
  NyxID usage receipt that the current transport does not provide.
- Deposited Lean projects are preserved and can target existing confirmed
  public statements. Arbitrary project execution is outside the verifier.
- Computational attachments are evidence delivered to reviewers. An explicit
  `mathesis.rational-projection-amgm.v1` declaration requests the bounded exact
  arithmetic replay described in [COMPUTATION_REPLAY.md](COMPUTATION_REPLAY.md).
  Other attachments are not executed; no upload alone establishes manuscript
  correspondence, formal coverage or a Lean verification receipt.
- Confirmed public error reports are highlighted; a complete editorial
  correction/retraction lifecycle and mathematical quality benchmark remain
  future product work. Private postpublication audits do not silently rewrite
  published inputs or expose private findings.
- GitHub Pages is a static showcase. A working hosted service still needs its
  API, MongoDB, worker, verifier, authentication and external model resources.

### Automatic image publication

After a push to `develop` or `main` passes the required `CI` workflow, `Publish images after CI` checks out that exact tested commit and publishes digest-addressable `mathesis-web`, `mathesis-api`, `mathesis-lean` and `mathesis-verifier` packages to GHCR. It also publishes a branch tag for operator use and stores immutable image receipts as workflow artifacts. The workflow never publishes a commit whose CI run failed; a manual dispatch is available for an explicitly selected ref.


## Shared request synchronization

The Rust worker reads canonical `trureturing-requests` hourly and keeps the last
complete snapshot on failure. Bootstrap/import routes and the read-only CLI are
in [API.md](API.md#shared-formalization-requests). Supply a mounted public-read
GitHub token via `WISHPOOL_REQUESTS_GITHUB_TOKEN_FILE` if traffic exceeds the
anonymous quota. The token is never sent to a source-provided URL.

All merged contributors keep attribution. Competitive ranking requires a verified
external identity in `WISHPOOL_REQUESTS_RANKED_LOGINS`, and is always refused for
request authors and `WISHPOOL_REQUESTS_UNRANKED_LOGINS`. This prevents
unknown author/coauthor GitHub identities from automatically receiving points.
For CLI bootstrap the corresponding eligibility environment variable is
`MATHESIS_REQUESTS_RANKED_LOGINS`; it defaults to empty. Review source authorship
before adding an identity; do not equate the issue creator with every author.
An administrator importing a curated snapshot is responsible for the same rule.

There is no dedicated collected-proof CMA sandbox or proof-generation queue.
Startup supersedes only obsolete `collected_formalization` jobs that are waiting,
failed or whose leases expired; proofs, receipts, active leases, exposition and
review histories are retained.
