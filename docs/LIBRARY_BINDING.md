# Prepared libraries and library-based proofs

The independent verifier checks a submitted Lean proof against an exact target
in a prepared workspace. Historically that workspace offered Mathlib only, so a
proof that already exists in another Lean library had to be ported into
submitted modules before it could be checked. A verifier workspace may now
declare further prepared libraries. A solution can import them, and the receipt
reports which library declarations the checked proof references.

This document defines the declaration, the import rule, the receipt report and
its exact meaning, the public projection, and the library-first behaviour of
automatic collected-paper proof jobs.

## Scope and unchanged rules

- Only a verifier-passed proof or disproof of the exact confirmed target counts
  (CLAUDE.md §12). A library import changes what a solution may reference, never
  what is checked: the Check module still compares the solution constant with
  `_root_.wishpool_target_prop` and admits only the three standard axioms.
- The report described here does not change admission, coverage, credit or
  leaderboard rules. It is not a recognized binding certificate (CLAUDE.md §15):
  ordinary Lean verification still cannot qualify a claim as one.
- A workspace without a declaration file behaves exactly as before. Receipts of
  projects that import no library are byte-identical to earlier receipts.

## Workspace declaration

The operator who prepares the verifier workspace may place
`mathesis-libraries.json` at its root:

```json
{ "libraries": [{ "package": "trureturing", "roots": ["D5", "Trureturing"] }] }
```

`mathesis_verifier::libraries(workspace)` reads it and binds every package to
the entry of the same name in the workspace `lake-manifest.json`:

- `package` is the Lake package name: 1–64 characters of `[A-Za-z0-9_-]`,
  unique, never `mathlib`. At most eight libraries; the file is at most 16 KiB
  and unknown fields are refused.
- `roots` lists 1–16 top-level module names owned by the library. Each is one
  Lean identifier, unique across all libraries, and none of the reserved roots
  (`Target`, `Solution`, `Check`, `Mathlib`, `Init`, `Lean`, `Std`, `Lake`,
  `Batteries`, `Aesop`, `Qq`, `ProofWidgets`, `Plausible`, `ImportGraph`,
  `LeanSearchClient`, `Cli`, compared case-insensitively).
- The manifest entry must pin a full 40-digit Git revision. Its `url` is kept
  when it is an `https://` address of at most 300 characters without whitespace
  (a trailing `.git` is removed). The address is reported, never fetched.
- `workspace_ready` additionally requires built artifacts of every declared root
  under `.lake/packages/<package>/.lake/build/lib/lean/`.

A request cannot add a library: the workspace alone decides which exist.

`GET /environment` of the verifier service returns the bound libraries as
`libraries: [{ name, revision, roots, url? }]`, omitted when there are none.
`Environment::label()` is unchanged, so targets and receipts still carry only
the `leanprover/lean4:vX, Mathlib <revision>` pin.

## Import rule

In the verifier:

- `Solution` and submitted supporting modules may import a module equal to or
  below a declared root, one module per `import` line. The module must have been
  built by the declaring package itself; otherwise the check is rejected with
  `missing prepared dependency: <module>`.
- `Target` never imports a library. A target stays readable over Mathlib and its
  own inlined definitions.
- A submitted module can never be named below a library root (compared
  case-insensitively). Library modules are imported, never resubmitted.
- The restricted-construct scan applies to submitted sources only. A prepared
  library is compiled by the operator and may use constructs such as
  `set_option` that submitted modules may not contain.

Layer 2 reads the declaration through the `ports::Verifier::libraries` port
method (default: none; implemented by the process and HTTP adapters, the latter
with a 16 KiB bound on the environment body). It does not depend on the verifier
crate. The same rule is applied before any verifier work:

- attempt admission, collected proposals and continuations, and private research
  attempts refuse a supporting module named below a declared root
  (`validate_lean_modules_with`);
- deposit and project preflight (`POST /api/v1/formalizations/preflight`, and the
  preflight inside deposit verification) accepts imports below declared roots
  instead of reporting `missing import …`, reports a module named below a root,
  and adds a warning naming the library and its revision. Preflight cannot check
  that an imported library module exists.

When the verifier declares no library, cannot be read, or reports a declaration
that fails Layer 2's bounds, these rules and their messages are exactly the
historical ones. The verifier remains the authority on every check.

## Receipt report

When the checked project imports at least one library module, the receipt gains

```json
"library_use": {
  "libraries": [{ "name": "trureturing", "revision": "<40 hex>", "url": "https://…" }],
  "declarations": [{ "library": "trureturing", "module": "D5.S3.…", "name": "D5.S3.….result" }],
  "total": 4,
  "complete": true
}
```

The trusted Check module produces it from the elaborated environment. Starting
at `wishpool_solution` (or `wishpool_disproof`) it visits the declarations of
the local modules (`Target`, `Solution` and submitted modules) and collects every
constant referenced by their statements or proof terms whose declaring module
lies below a declared root. Only the output of compiling the Check module is
read; the output of submitted modules is discarded.

Exact meaning:

- **Direct references.** A declaration is listed when a local statement or proof
  term references it. Declarations used only inside the library's own proofs are
  not listed, and Mathlib constants are never listed.
- **Provenance by module, not by name.** A local declaration that merely reuses a
  library-looking name is not reported, because no submitted module can be named
  below a library root.
- **`declarations`, `total`, `complete`.** `declarations` is sorted and holds at
  most 64 entries; `total` counts all referenced library declarations, including
  unlisted ones and names that cannot be carried as plain text. `complete` is
  false when the bounded walk stopped before visiting every reachable local
  constant; further declarations may then be referenced.
- **A revision pin.** `libraries` names every imported library with the exact
  revision of the workspace that performed the check.

Limits:

- It is not a semantic-equivalence statement. It says that the checked target
  follows from the listed declarations together with the submitted Lean. It does
  not say that a library declaration states the same mathematics as the
  manuscript or as the target; target faithfulness remains a separate inspection.
- It is not a coverage certificate, a recognized binding certificate, a
  Trureturing registration or a freeze status, and it creates no proof-use edge
  in the research graph.
- An imported library whose declarations the proof never references yields a
  report with `total: 0`.

Layer 2 mirrors the report on `VerificationReceipt.library_use` (absent when
`None`, so stored historical receipts and existing JSON stay byte-compatible).
Before a receipt is saved, a report that is unbounded or inconsistent (unknown
library, more listed than `total`, malformed revision or address) is refused as
an invalid verifier receipt. Target-elaboration receipts must carry no report.

## Public projection

Layer 2 derives `proof_basis` from the saved receipt alone:

| `kind` | Meaning | Public label |
|---|---|---|
| `library_declarations` | A verified receipt with `total > 0`: the checked target follows from the listed library declarations at the pinned revisions together with the submitted Lean | Proved from library declarations |
| `new_lean` | A verified receipt whose proof references no prepared-library declaration | New Lean proof |

Unverified work has no `proof_basis`. The `library_declarations` object repeats
`libraries`, `total`, `complete` and lists each declaration with an optional
`source` link. The link is built only from the receipt's library `url`, its
revision and the module path, as `<url>/blob/<revision>/<Module/Path>.lean`. It
is produced only for a `https://github.com/<owner>/<repo>` address and assumes the
library's Lean sources start at the repository root; any other receipt carries
no link.

`proof_basis` appears on:

- every public verified attempt (`PublicAttempt`): the statement and conjecture
  detail, a paper's `claims[].verified`, collected-paper proofs, and the
  `GET /api/v1/papers/{record}/statements/{claim}/attempts` and
  `GET /api/v1/conjectures/{record}/{claim}/attempts` lists;
- the owner or public attempt view (`GET /api/v1/attempts/{id}`), once verified;
- formalization directory rows (`GET /api/v1/formalizations/opportunities`), for
  the winning receipt; `basis=library_declarations|new_lean` filters rows by it
  and never matches a row without a verified proof;
- the verification bundle, inside `attempt`.

The web Formalizations page and the result, statement and conjecture readers show
the label and the declarations; the paper page shows the label and the pinned
library for each verified result. The directory offers the two verified kinds as
a filter beside the status filter. Interface copy says “proved from library
declarations” and “follows from”. It never describes a library declaration and a
manuscript statement as equivalent.

`new_lean` describes the checked sources, not authorship: a proof ported from a
paper's existing formalization into submitted modules is `new_lean` and keeps
its separate attribution.

## Verification bundles

A bundle contains the exact target, solution and submitted modules. It does not
contain library sources. Rechecking a bundle whose receipt has `library_use`
therefore needs a prepared workspace that declares and contains every listed
library at exactly the recorded revision, in addition to the pinned Lean/Mathlib
pair. The bundle's `instructions` say so; `schema_version` is unchanged.

## Library-first collected proof jobs

Automatic collected-paper formalization pins the verifier environment before
any provider admission. A new job now pins, in the same revision-fenced write,
the Lean/Mathlib pair and the prepared libraries the verifier declares at that
moment (`CollectedProof.libraries`, via `pin_collected_environment`). A saved pin
is never replaced.

When the pinned environment declares libraries, the preparation and proof
prompts carry one additional line instructing the agent to

1. first check whether the paper's pinned Lean module exists below a declared
   root in that library at the pinned revision;
2. if it does, write the faithful Mathlib-only target and prove it in `Solution`
   by importing that library module and applying its existing declaration,
   proving any needed equality between the target's inlined definitions and the
   library's definitions by unfolding both (`rfl` alone may fail);
3. port into modules, or newly prove, only what the library lacks.

The line is rendered only from the durable pin, so retries render identical
bytes even if the verifier is redeployed with another library revision. The
toolchain validation of proposals and the `reused_from` rule are unchanged: a
proof that applies the paper's existing formalization still names the paper's
pinned Lean origin, keeps its formalizers and license, and earns no new proof
credit.

Already admitted work keeps its exact prompt bytes:

- legacy work admitted without an environment pin renders the legacy prompts;
- work pinned to a Lean/Mathlib pair without a library, whether pinned before
  this change or on a verifier that declares none, renders the previous pinned
  prompts and never acquires a library pin.

Both cases are fixed by golden fixtures captured from the renderer that admitted
that work (`api/crates/mathesis/src/review/referee/fixtures/collected-*.txt`).
Each admitted run is additionally fenced on the SHA-256 of its prompt, so a
changed rendering would fail with `CMA inputs changed after admission` rather
than reach the provider.

The guidance in step 2 follows a measurement of 2026-11-11 with Lean v4.35.0
against a local build of the upstream library: a Mathlib-only target for the
result of the collected paper `oeis-a378495-difference-conjectures`, proved by
importing `D5.S3.Combinatorics.Derangements.AdjacentDifferenceRefutation` and
applying its theorem `result`, was returned as `proved` with the three standard
axioms and a report listing that theorem and three definitions. Equalities
between the target's inlined definitions and the library's definitions were not
provable by `rfl`; unfolding both definitions followed by `congr`/`rw` worked.

## Deployment order

`Environment` refuses unknown fields. An API or worker built before this change
cannot read an `/environment` response that lists libraries and would treat the
verifier as unavailable. Deploy in this order:

1. API and worker with this change. Against a verifier without a declaration
   they behave exactly as before.
2. A verifier image whose workspace declares libraries.

A verifier with this change and no declaration file serves the historical
`/environment` body and can be deployed at any point. When
`WISHPOOL_VERIFIER_LEGACY_URL` is configured, requests are routed by the
Lean/Mathlib pin only; a legacy verifier is not consulted for libraries.

Changing the library revision of a deployed verifier does not rewrite anything:
saved receipts keep the revision they were checked against, and admitted
collected jobs keep their pinned revision in their prompts. A proof that depends
on a declaration absent from the new revision is rejected by the verifier with
its ordinary compiler message.

## The reference image's library

`infra/verifier/` prepares one library beside Mathlib: the Lake package
`trureturing` (<https://github.com/the-omega-institute/trureturing>) with module
root `D5`. Four files define it:

| File | Content |
|---|---|
| `lakefile.toml` | the `[[require]]` entry with the repository address and one full 40-digit revision |
| `mathesis-libraries.json` | the workspace declaration, copied to the workspace root |
| `library-modules.txt` | the modules to compile, one per line; currently the Lean module of each collected work |
| `library-proof.json` | one proof of a Mathlib-only target that needs a declaration of the library |

`Lean.Dockerfile` builds the workspace in this order:

1. It fetches the library at the pinned revision with only the `D5` sources
   checked out, then runs `lake update`. Lake finds the package at that revision
   and writes its address and revision to `lake-manifest.json`, the file
   `libraries()` reads.
2. It fetches the Mathlib build cache as before.
3. It compiles the listed modules and their imports from source with the pinned
   toolchain and the library's own Lake options. Nothing is downloaded prebuilt
   for the library. A listed module that does not compile at the pinned revision
   fails the build. Lake's output goes to a file, because the library's linter
   messages exceed the image builder's 2 MiB step log: the step prints one
   progress line a minute, and on failure the failing modules with their errors.
4. It removes the library's sources, Git data and intermediate C files. Checking
   a proof reads compiled modules only, and every job copies the workspace.

A module of the library outside the compiled closure is absent from the image.
A solution that imports it is refused with `missing prepared dependency`.

The verifier image build runs
`real_lean_prepared_library_proves_a_mathlib_only_target` with
`WISHPOOL_TEST_LIBRARY_PROOF` naming `library-proof.json`. The test checks the
proof in the prepared workspace and requires the receipt to report the library's
manifest revision and the named declaration. A workspace that declares a library
without that input fails the test.

### Changing the revision or the module list

The revision, the module list and the proof change together in one pull request:

1. Set the new revision in `lakefile.toml`. One image carries one revision of
   the library; collected works may pin other revisions in their own records.
2. Add or remove modules in `library-modules.txt`. Every listed module must
   exist at that revision.
3. Keep `library-proof.json` valid: its module must stay inside the compiled
   closure and its declaration must keep its name and statement.

The Lean base image is addressed by a pin: the first twelve hexadecimal digits of
the SHA-256 of `lean-toolchain`, `lakefile.toml`, `mathesis-libraries.json`,
`library-modules.txt` and `Lean.Dockerfile`, concatenated in that order. A change
to any of them produces a new pin, and the library is compiled again:

- The first CI run of a new pin compiles the library inside the verifier job.
- `publish-images.yml` publishes the base as `mathesis-lean:<pin>` when no image
  of that pin exists. Dispatching it for the pull request branch with the
  verifier option publishes the base before the merge.
- Every later CI and publication run of the same pin reuses the published base.

Readings on GitHub-hosted runners for revision `542ba53d78db` with the 39 listed
modules (2026-11-10 and 2026-11-11: two builds, then one CI run on the published
base):

| Reading | Value |
|---|---|
| Modules compiled: the listed modules and their imports | 537 |
| Compiled library kept in the image | 1,074 MiB |
| Compilation of the library | 5,221 s |
| Whole base build step | 5,336 s and 5,872 s |
| Real Lean tests of the verifier image, six tests | 1,278 s, 1,410 s and 1,994 s |
| The same tests before the library, five tests on the Mathlib-only base | 969 s |
| Verifier CI job that compiles the library | 2 h 07 min |
| Verifier CI job on the published base | 42 min |

## Current state

The verifier image built from this repository declares `trureturing` with root
`D5`. A deployment offers the library once its verifier service runs such an
image, after the API and worker of the deployment order above. Until then its
receipts are `new_lean` and no job is library-first. This change adds no
configuration key.
