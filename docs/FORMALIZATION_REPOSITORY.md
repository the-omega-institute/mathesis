# Mathesis formalization repository

The `mathesis-papers` organization owns its formalization content repository,
`formalization`. Mathesis is the authority: platform state lives in MongoDB,
independent verifier receipts establish checked status, and the public
leaderboards establish credit. This repository is a projection rebuilt from
public Mathesis routes. A pull request, a source link, target elaboration or a
successful repository build confers no verification or credit (CLAUDE.md §5).

For each current paper, the projection separates statements proved from the
existing formal base, statements verified with newly supplied Lean, confirmed
open targets, and statements without a target yet. The existing base consists
of Mathlib and the prepared `trureturing` library at the verifier's exact pins.
A `library_declarations` receipt says the target follows from its listed library
declarations together with the submitted Lean. It is not semantic equivalence,
a coverage certificate or a Trureturing registration. `new_lean` describes the
checked sources, not novelty or original authorship. A paper-level Lean link
alone never places every extracted statement in the covered category.

Finished sources can be cloned and compiled here independently of publication
in trureturing. Write-back goes through the trureturing request queue,
<https://github.com/the-omega-institute/trureturing-requests>. This package
implements the projection and publisher integration, not queue submissions.

## Public snapshot and deterministic rendering

The existing atomic snapshot transaction now also collects:

- The statement detail and byte download of every directory node with a
  confirmed target, including its exact Lean text, digest and toolchain.
- Every public verified attempt from the statement's attempts route and its
  `GET /api/v1/papers/{record}/statements/{claim}/attempts/{attempt}/bundle`.
  Later “also verified” attempts and disproofs remain separate artifacts.
- Every page of `/api/v1/research-problems`, then each problem's public detail.
  Only the versioned `curated` record and summary are needed for this projection;
  associated note pagination adds no curator target. Only `state: elaborated`
  targets expose text and become Lean modules. Checking/refused metadata stays
  in the snapshot and is not promoted to an open target.
- Both `/api/v1/leaderboard?period=all&entrants=all` and
  `/api/v1/leaderboard/formalizations?period=all&entrants=all`, preserving public
  row order and all returned fields. Repository nodes never recompute credit.

Extra data is stored in `raw/formalization.json`, schema
`mathesis.formalization-snapshot.v1`. Target digests, public receipt target,
solution and module digests, toolchain, work/version/claim/attempt identities and
source agreement are checked before commit of the snapshot. Any unavailable
source or mismatched bundle aborts the transaction and preserves the old
snapshot. A snapshot is a traversal interval, not a MongoDB transaction; a
concurrent target/attempt change can require a fresh run. Accepted/private scope
rules remain those of §31: this content tree publishes current in-scope
CC-BY-4.0 collected works only; other directory rows remain in the catalog.

The renderer has no network or clock input. It renders `formalization/` alongside
`catalog/`, `profile/`, `papers/`, `template/` and `site/`. Its complete inventory
and SHA-256 values enter `.mathesis/rendered.json`; `mathesis-catalog check`
verifies that inventory and recomputes formalization classifications, counts
and source bindings as well as the existing catalog statistics. The tree has
its own `.mathesis/generated.json` for an offline clone check. Hash checks show
projection integrity, never independent verification of the mathematics.

## Generated content and indexes

```text
formalization/
├── README.md
├── LEADERBOARD.md
├── leaderboard.json
├── snapshot.json
├── lean-toolchain
├── lakefile.toml
├── lake-manifest.json
├── mathesis-libraries.json
├── environment/
│   ├── verifier-pins.json
│   └── verifier/                 # exact source verifier inputs
├── targets/<work>/v<version>/<claim>/
│   ├── Target.lean               # exact confirmed text, no wrapper
│   └── target.json
├── curated/<research-problem-id>/v<version>/
│   ├── Target.lean               # exact curator text
│   ├── problem.json
│   └── README.md
├── proofs/<work>/v<version>/<claim>/<attempt>/
│   ├── Target.lean
│   ├── Solution.lean
│   ├── <supporting/module>.lean
│   ├── receipt.json
│   └── bundle.json               # complete public bundle, including attribution
├── disproofs/<work>/v<version>/<claim>/<attempt>/  # same layout
├── index/
│   ├── nodes.json
│   ├── curated.json
│   ├── modules.json
│   └── papers/<work>.json
├── ci/mathlib/{lean-toolchain,lakefile.toml,lake-manifest.json}
├── scripts/{check.py,build.py}
└── .github/workflows/check.yml
```

The four source areas have generated READMEs even while empty. JSON serialization
preserves the full bundle content; extracted Lean files preserve the exact UTF-8
source bytes, including line endings. Contributor Lake configuration is never
executed. Supporting dotted module names map to slash paths and are checked for
traversal, reserved names and case collisions before writing.

`index/nodes.json` has schema `mathesis.formalization-nodes.v1`. Nodes keep §31
identity, lineage, statement, original authors, contributor, attribution,
receipt, public proof basis and paper-level Lean source. Added fields are:

| Field | Meaning |
|---|---|
| `classification` | `covered_by_existing_base`, `newly_formalized`, `open_target`, `no_target_yet` or `disproved` |
| `reason` | Exact public API `next_step` for `no_target_yet`; null otherwise |
| `target_path` | Confirmed exact module path, or null |
| `results` | Public attempt id, path, verdict, `also_verified` and proof basis for each bundle |
| `request_queue_url` | The trureturing-requests repository URL above |
| `trureturing_write_back.request_url` | Nullable public request URL |
| `trureturing_write_back.state` | Nullable publicly reported state; no inferred transition |
| `trureturing_write_back.revision` | Nullable verified destination revision |

All three write-back values are null today: the existing public routes provide
no such sidecar. No request id, state or revision is derived from a filename,
Lean source link or receipt. A future public contract must supply them before
the generator can populate them.

Each `index/papers/<work>.json`, schema `mathesis.formalization-paper.v1`, keeps
paper/version/authorship/source identity and the same five `split` arrays of
full node records. Empty arrays are explicit. Covered nodes retain declarations,
source links, library revisions, `total` and `complete` in `proof_basis`, while
Lean/Mathlib pins remain on the node and in the receipt. A directory's winning
receipt determines its classification even if other attempts have a different
verdict. Proof and disproof bundles are both retained.

Curated targets have their own index, area and scope. They are the curator's
formalizations, not confirmed by the original posers. Elaboration establishes
that the proposition is well formed; it establishes neither faithfulness nor a
proof. They open no proof submission, solution verification or leaderboard
credit (§23).

`index/modules.json` lists every standalone target and bundle build unit, its
exact toolchain and compatibility with the generated default environment.
`leaderboard.json` has schema `mathesis.formalization-leaderboards.v1`, snapshot
time, route names, `period: all`, `entrants: all` and both unchanged row arrays.
The Markdown board displays rank, entrant, Others, Total, proved, disproved and
last-solve time; the JSON retains every field.

## Verifier pins and compilation

`sdk/catalog/scripts/refresh-verifier-pins.py` reads `infra/verifier/` in the same
Mathesis checkout. It performs anonymous GETs of dependency manifests at the
exact direct revisions, checks conflicts, discovers the packages' Lake config
filenames, and generates `sdk/catalog/verifier-pins.json`. No Lean source tree
or build cache is downloaded by this refresh. The frozen file retains exact
verifier inputs and their hashes, upstream lock origins and hashes, full
resolved Lake locks, a Mathlib-only lock and the reference image tag. Direct
requirements come from the verifier Lake file, never manually copied pins.
The SDK build includes the source verifier inputs and refuses stale generated
pins; source builds therefore use the full Mathesis checkout.

The generated root `lean-toolchain` and Lake file are the verifier's exact
bytes. Its Lake manifest pins Mathlib, the prepared library and transitive
packages at full revisions. The generated `ci/mathlib/` project omits the
prepared library and retains the pinned Mathlib closure. Refresh and commit the
frozen metadata whenever verifier inputs change; rendering uses only that
frozen metadata.

All targets declare the same `_root_.wishpool_target_prop`. There is no aggregate
Lean import and no default `lake build` claiming to build the content. With the
pinned toolchain installed, a clone prepares Mathlib with:

```sh
cd ci/mathlib
lake exe cache get
cd ../..
python3 scripts/check.py
python3 scripts/build.py targets
# Or choose one exact target path from index/modules.json:
python3 scripts/build.py curated/<id>/v<version>/Target.lean
```

The target command compiles all compatible confirmed and curated target files,
one temporary module root per target. It explicitly reports targets with other
historical pins as skipped. A selected historical target requires an existing
matching `--workspace`; a mismatch fails. No proposition is proved by these
commands.

For one proof/disproof bundle, prepare the root workspace with its pinned
Mathlib cache and compile the required prepared-library imports from source at
the recorded revisions. The reference module list is included in
`environment/verifier/library-modules.txt`; the reference build takes about
87 minutes. Then run:

```sh
python3 scripts/build.py proofs/<work>/v<version>/<claim>/<attempt> --workspace .
# An existing matching prepared workspace avoids rebuilding dependencies:
python3 scripts/build.py disproofs/<work>/v<version>/<claim>/<attempt> \
  --workspace /path/to/prepared/workspace
```

The runner checks Lean/Mathlib and every receipt library pin, copies the unchanged
sources into a temporary module root, compiles local dependencies in topological
order, then removes temporary artifacts. This checks compilation only. The
independent verifier also enforces restricted constructs, exact target/solution
types and the standard axiom set; use each unchanged bundle's `instructions`
and `verification_request` for that replay. A historical verifier binary revision
is not recorded in the public bundle, and this repository does not invent one.

## CI and anonymous image access

At `2026-11-13T07:03:46.919580+00:00` the exact image tag derived from the verifier
inputs was `558d9c06cdcf`. Anonymous GETs returned:

| Request | Result |
|---|---|
| `https://ghcr.io/v2/the-omega-institute/mathesis-lean/manifests/558d9c06cdcf` | HTTP 401, Bearer challenge for repository pull scope |
| `https://ghcr.io/token?service=ghcr.io&scope=repository:the-omega-institute/mathesis-lean:pull` | HTTP 401, authentication required |

Anonymous pull access was therefore unavailable from this machine. This does
not establish whether the image is private, absent or restricted by registry
policy. No authenticated registry request or layer download was made.

The generated workflow runs on push, PR and manual dispatch with read-only
contents permission and a pinned checkout action. It checks the projection,
installs the pinned Lean toolchain within the ephemeral job, obtains the pinned
Mathlib cache, and compiles compatible targets separately. A final job summary
states its limits even on failure. It does **not** pull that image, build
trureturing, compile proof/disproof bundles, run the independent Mathesis
verifier, establish natural-language faithfulness, verify credit or publish
anything. A missing Mathlib cache or toolchain fails the job; it is not reported
as a passed compile. Hosted execution of the workflow remains to be observed.

## Publisher and dispatcher handoff

The external catalog publisher stages snapshot → fetch → render → check, then
rerenders/checks with its publication plan. It additionally runs the clone's
`formalization/scripts/check.py` and validates the projection schema before
sealing the stage. A source pin predating this renderer fails before any remote
write. Formalization output enters the same stage seal as catalog/profile/site.

Apply requires an existing, active, public `mathesis-papers/formalization`
repository and reads its head with the other shared destinations before writes.
It does not create this repository. Synchronization uses the existing atomic,
non-forced per-repository ref update and skips unchanged bytes. It manages only
the generated root files, this workflow and two scripts, and the source/index/
environment/CI namespaces. Other files and workflows are preserved. A stage
failure publishes nothing; an apply failure resumes the completed per-repository
steps on retry. Profile and org site link the formalization repository. It is
reserved against use as a per-paper repository name.

The publisher branch starts from `origin/main`. Its checked-in `mathesis_commit`
remains the previously published `b275c9bca206bf6befa365f7b960b1b5290a60a9`:
the dispatcher, who commits this Mathesis worktree, must replace that pin with
the actual reviewed SDK commit and make it reachable before running the remote
publisher. A future commit SHA is deliberately not fabricated. This package
makes no push, repository creation, GitHub App request or production write.

## Real local run, 2026-11-13

The anonymous public snapshot captured at `2026-11-13T07:11:30.236956Z` contains
39 collected works, zero accepted records and 229 distinct current statement
nodes. All 229 directory rows are preparing; there are zero confirmed/open
targets, zero verified proof/disproof bundles, zero covered-by-base nodes and
zero new-Lean nodes. All 229 `no_target_yet` reasons are preserved from the API.
All 39 paper indexes have empty covered/new/open/disproved arrays. There are
39 research-problem listing rows, 28 curated records, and exactly seven curated
records with elaborated Lean targets. Both public all-time/all-entrants
leaderboards have zero rows. The rendered repository has seven Lean files.

The output at `/Users/lexa/Desktop/lexa/omega/mathesis-formalization` is initialized
on `main` with no remote and a local snapshot commit. Complete snapshot
and render evidence is retained at
`/Users/lexa/Desktop/lexa/omega/mathesis-evidence/2026-11-13-formalization-repository`.
The public OpenAPI was also read at `/api/v1/openapi.json` (OpenAPI 3.1.0,
SHA-256 `45a19344eb61520baf7dbd258b99a4617cc0630a2c6eddc113e52cc166dac22c`).

Before Lean work, the basename comparison of every `ps` `comm` field found no
running `lean` or `lake`; memory-pressure output reported 79% system-wide free
memory. The existing local `/Users/lexa/Desktop/lexa/omega/trureturing` workspace
has Lean `leanprover/lean4:v4.35.0`, Mathlib at
`56a005855ea17e0b0958f73ec801ae140b0a1679`, and a compiled `Mathlib.olean`.
The following command compiled the exact version-2 curator target for
Alekseyev's conjecture on the last two base-p digits successfully:

```sh
python3 scripts/build.py \
  curated/e727ed23b6a60bf721ed035c9e8e8f02fcda77fe1376b972fab384fe732619f3/v2/Target.lean \
  --workspace /Users/lexa/Desktop/lexa/omega/trureturing
```

This was one target elaboration, with no new verifier receipt, proof or credit.
No Mathlib download, global install or library rebuild was performed. The other
six target files were not locally compiled in this run. PID 764 was not touched.

## Next package: separate SolveFile stores (design only)

Today `SolveFile` holds target/preparation state, attempts, source references,
provider runs and authorization. `find_attempt`, `solve_credits`, contribution
boards, profiles and reconciliation scan `solving.all()`. The formalization
candidate adapter also traverses aggregate state and source checks for public
rows. This package changes none of those runtime stores or routes.

The next package should introduce four distinct persistence contracts, while
retaining a small revision-fenced orchestration header for generation allowance,
leases, admitted provider runs and cleanup handles:

| Store | Identity and contents | Required access |
|---|---|---|
| Targets | Immutable exact text/digest, Lean/Mathlib/library pins, source/version/claims revision/source and statement hashes, faithfulness and confirmation/admission provenance; separate current pointer | By full source identity or target digest, with explicit current/invalidated status |
| Attempts and receipts | Attempt id, entrant/owner, exact target/source pin, solution/module GridFS refs and hashes, intermediate/reuse context, immutable independent receipt and invalidation | Indexed attempt lookup and per-target ordered attempts; private owner listing |
| Credit ledger | Eligible winning receipt identity and timestamp, source ownership, entrant/agent owner, proof/disproof kind, paper/conjecture scope, reuse exclusion and public-current eligibility generation | Indexed all/month and people/agents projections, per-entrant profile; no scan of source blobs |
| Directory rows | Public current-source mathematics, target pointer or null, preparing/open/verified/disproved state, next step and receipt/basis references, visibility/source generation fence | Indexes for kind/status/basis and stable cursor; filtering before pagination |

The directory is disposable and rebuildable from authoritative stores. The
credit ledger is an idempotent eligibility projection of receipts, not a new
way to grant credit. Keep credit history and present public eligibility separate:
a visibility change removes public eligibility without deleting receipts or
privately retained attribution. Do not let delayed projection updates leak a
private/stale reused source; validate a current public-generation fence at read
admission as well as during projection updates. Count materialization must use
the same fence so lists, counts, profiles and exports agree.

The split must preserve all of these §12/§28/§29 invariants:

- Exact current source/version/claims revision, target digest and toolchain bind
  admission and completion. Collected source and statement hashes, author
  approval/confirmation or collected semantic admission, and independent
  elaboration remain separate gates. Curated targets never become submission
  targets through migration. Target correction/visibility/source changes
  invalidate pointers and pending work without rewriting historical evidence.
- Only a credential-free independent verifier can attach a successful receipt.
  Preserve fresh isolated module builds, exact imported target type, supporting
  source binding, permitted imports and the three-axiom rule. A live lease and
  source/target/reuse validity are checked again before the revision-fenced
  receipt write. Nothing in the projection trusts claimed success.
- Preserve immutable attempt identity, entrant/target/solution idempotency,
  intermediate and reuse distinctions, per-entrant limits, durable enqueue and
  crash recovery. Keep source bytes/GridFS hashes and attribution intact. Never
  recompute a receipt's timestamp, digests, library report or historical pins.
- Earliest independent verifier timestamp wins, with attempt id breaking ties;
  later successes remain “also verified”. Intermediate propositions grant no
  final-proof points. First checked paper proofs count only on Formalized;
  conjecture proofs/disproofs count on the conjecture board. Imported existing
  venue work earns no fresh credit. “Others” follows submitter, linked coauthor
  and agent-owner identity; Total includes own work. Keep UTC month semantics,
  people/agent filters and exact sorting by Others, Total, last solve, entrant.
- Preserve §28's one durable collected proof job per current paper version,
  reconciliation and interrupted admission recovery, independent semantic check
  and kernel verification, immutable library/Lean/Mathlib pins before provider
  admission, library-first guidance and original prompt bytes. A model outage
  retains its original job/remote identity and does not spend a generation.
  No migration triggers a manuscript re-review or upgrades a historical pin.
- Authorization expiry, pause/resume, consumed lifetime generations, admitted
  input bytes, original managed handles and explicit target rebinding keep their
  existing fences. Split transactions must not spend a round twice, revive
  expired work, grant a fresh allowance or lose asynchronous cleanup ownership.
- Public reads expose only currently public/current targets and successful
  receipts; pending/rejected/private attempts and private runs stay hidden.
  Preparing rows expose mathematics/authorship with null target fields and no
  credit. Only confirmed targets offer direct submission. Default directory
  calls remain confirmed-only; `unverified`/`preparing` retain their broader
  semantics. Discovery never enqueues/restarts provider work. Proof basis stays
  receipt-derived and source publication remains separate from verification.

Migration order should be explicit and reversible:

1. Add versioned models, indexed stores and compatibility readers with all
   historical serde defaults. Define unique compound keys and stable cursor
   order. Save an immutable source snapshot for differential comparisons; do
   not infer absent legacy pins, faithfulness checks or credit.
2. Backfill immutable targets and attempts/receipts with original identifiers,
   blobs and digests from revision-fenced aggregates. Persist a checkpoint and
   aggregate revision for each copy; reread changed aggregates. Keep SolveFile
   authoritative, and retain its provider/authorization coordination fields.
3. Add transactional outbox or same-Mongo-transaction writes tying each
   authoritative mutation and projection event to one revision. Make event
   application idempotent, ordered per source and replayable; cover the
   enqueue-after-save and receipt-before-projection crash windows before any
   reader switches. Partial dual writes must not expose new eligibility.
4. Replay directory and ledger projections from exact current source checks and
   independently saved receipts. Differentially compare every node identity,
   target, winner, reason, basis, visibility and board/profile value against the
   old implementation, including ties, mixed verdicts, own work, reuse becoming
   stale/private, source updates and historical pins. Add bounded indexed
   query/scan instrumentation; no full-scan fallback for an unbuilt projection.
5. Switch indexed attempt lookup first, then directory reads, then
   leaderboard/profile/count/export reads after shadow parity. Keep a fenced
   compatibility rollback path and replay checks; publish no retrospective
   credit for copied sources. Move target/attempt writes only once their
   transactions and leases preserve the old admission/completion fences.
6. After durable parity and recovery validation, remove old scanning paths and
   duplicate aggregate payloads in a separate migration. Archive rather than
   erase unique historical evidence. Keep API shapes, public routes, receipt
   meaning and repository generation compatible throughout.

No store split, Mongo migration, credit rewrite or queue write-back is
implemented in this work package.


## Validation for this package

`sdk/catalog` passed `cargo fmt --all --check`,
`cargo clippy --locked --all-targets --all-features -- -D warnings`, and
`cargo test --locked` (23 tests). Tests cover exact target downloads, bundle
hashes and supporting modules, independent proof-basis splits, disproofs,
curated non-submission scope, public leaderboard preservation, offline
repeatability, atomic failure preservation, rehashed count corruption and
mixed-verdict winner preservation. The loopback fixture handles interrupted
reads and incomplete preconnections without falsely returning HTTP 404.

Publisher tests passed all nine tests, including the actual SDK/template/paper
staging integration using the three explicit local fixture paths. Ruby YAML
validation checked workflow triggers, read-only permissions, exact action pins
and every inline shell block; `bash -n` checked publisher shell scripts. The
formalization workflow received equivalent YAML and shell checks. Python source
syntax was parsed without installing packages. `actionlint` was unavailable;
its result is not claimed. The live rendered output passed SDK `check` and the
standalone clone checker. Logs and probe/compile evidence are retained in the
evidence directory above.

The hosted workflow, a clean dependency fetch/cache run, the full prepared-library
build, all six other curator target compilations, independent verifier replays
of future proof bundles, remote publication and request-queue write-back were
not performed. There were no public proof bundles to replay in the live snapshot.
