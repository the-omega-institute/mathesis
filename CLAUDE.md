# Mathesis

This file is the contract for anyone — human or agent — working in this
repository. Rules here are binding.

## 1. What Mathesis is

A venue where mathematicians submit their own papers, short notes and
conjectures through one entry and one review pipeline. The venue reads their
statements, audits the referee's mathematical reading, and gives accepted work
one shared `WP-<year>-NNNN` record sequence. Authors may instead choose an independent
private Mathesis review, which never applies publication policy or creates a record.
Short notes follow exactly the paper
pipeline; only their labels differ.

- **Input**: papers and notes upload PDF, `.tex`, `.zip` or `.tar.gz` manuscripts.
  PDF intake uses a bounded Poppler text read and numbered statement headings,
  preserves the original PDF bytes, and suggests bounded front-matter title/abstract.
  Ambiguous fields stay empty; authors confirm bibliographic metadata before review. Scans
  without text or unreadable statement boundaries are refused with an actionable
  explanation. PDF extraction warnings remain visible before confirmation.
  The web form uses Upload → Check PDF → Details → Authors → Submit tabs;
  source-derived metadata is editable after the PDF check. Preparation persists a
  private incomplete draft, using the ordinary durable compile lane. Its intake
  marker blocks ordinary statement confirmation until metadata is supplied. Final
  submission atomically confirms metadata/statements against the source version,
  digest and claims revision; exact retries do not create new review rounds.
  Legacy submissions and downstream manuscript delivery keep their existing paths;
  multiple source files are packed into one path-preserving ZIP.
  Conjectures use the same endpoint with either LaTeX source or a typed title,
  statement (at most 20,000 characters), optional background (50,000) and origin.
  Typed input becomes a minimal stored `.tex`, using the same reader, blob store,
  PDF build and author confirmation. Its single conjecture is a main claim.
- **Statements**: the author confirms kind, main or supporting role and
  dependencies. Papers/notes require a proved main result; conjectures require a
  main conjecture or question. Stored documents without `kind` load as `paper`.
  Extracted `has_proof` records recognized proof markup, not proof correctness or
  absence. A current source audit may establish a prose proof for formalization
  eligibility without rewriting confirmed statements. Recommendations retain such
  results; approval, target confirmation and independent verification remain separate.
- **Analysis**: for papers/notes, whether prior work states or implies a result,
  and whether its proof carries a live intermediate proposition or conclusion (an escape
  witness) or follows by binding alone. Conjectures are checked for well-posedness,
  open/known status, and whether a proof would carry new mathematical content.
  Named works are reported names, never verified bibliographic claims.
- **Private review**: `purpose: review_only` uses the same source/statement checks,
  referee and source audit, followed by advice and a private letter. `review_complete`
  means the audit is ready; the letter may still be running. This mode stays private,
  has no publication decision/record, volunteer tasks or accepted-only Lean probe.
  Optional `review_scope` (5,000 characters) gives untrusted author-requested focus.
  Absent purpose defaults to `publication`; its acceptance policy is unchanged.
- **Public layers**: the final-step checkbox “Make my results public once accepted.”
  defaults to checked for new submissions. Unchecking it selects `purpose: review_only`
  and private feedback without a publication decision; existing review-only drafts
  and legacy review links start unchecked. Purpose is chosen atomically with final
  metadata/statement confirmation, before review begins. Publication happens at the decision, before advice,
  letter or Lean. The author may later change visibility through the existing
  route. Public details contain title, authors, abstract, DOI and record id;
  statements with dependencies; witness lemmas of audited-correct main content
  results; and verified Lean artifacts. Lean ✓ means a verifier-passed proof.
  Correctness labels, comments, rationale, audit summaries, referee reports,
  letters and advice stay private to authors/staff. Private or legacy undecided
  records expose title, authors, kind and record id only.
- **Readable exposition**: every accepted manuscript queues a separate durable
  CMA job for one English and one Simplified Chinese plain-language explanation.
  Both describe the question, result, mechanism, significance and limits from the
  manuscript only; private review findings are never supplied. Exposition is not an
  admission or verification receipt. It is bound to the exact source version and
  confirmed statements. Authors see it privately; public readers see only the
  explanation of the accepted public version. Staff may backfill, retry or correct
  exposition without restarting the referee or creating manuscript versions.
- **After acceptance**: papers/notes receive the existing private post-letter
  Lean probe; publishing proofs still requires author approval. Accepted
  conjectures receive a Lean target and plain-language reading after the letter.
  The binary elaborates `Target.lean`: `import Mathlib`, confirmed definitions,
  and `def wishpool_target_prop : Prop := <statement>`. No proof hole is allowed.
  It stores exact text, SHA-256 digest and pinned Lean/Mathlib revision. Only the
  submitting author in a cookie session confirms the digest or rejects it with
  a correction. A new version voids confirmation. Legacy theorem targets are
  converted mechanically when possible, independently elaborated, and confirmed
  again; other legacy targets are regenerated.
- **Open problems from papers**: every author-confirmed conjecture/question in
  an accepted public paper/note that the paper does not settle is a candidate.
  A durable, version/claims-revision-fenced job runs a separate GPT Pro conjecture
  referee and Codex source audit for each claim, using the same sanitizers and
  Policy well-posed/open/content rule as submitted conjectures. Only passing
  candidates enter `/conjectures`, with their source record and claim id. The
  paper's submitting author confirms their exact Lean target. Private paper
  problems never enter the list. New versions require a fresh check. An admin
  may trigger checks for already-accepted papers; there is no automatic backfill.
- **Contributions**: Solve · Formalize · Report errors · Donate quota. Anyone
  signed in may submit a Lean attempt against a confirmed public target. Proved
  paper statements require author approval before target generation. Confirmed
  error reports earn separate profile credit. Donated NyxID quota is attributed
  to venue CMA probes, target generation and attempts by `wishpool-agent`, owned
  by the venue; Supporters lists only NyxID-reported usage, without ranking.
- **AI review requests**: uploading prepares a draft; clicking Submit for review
  requests AI review. Do not require an extra processing, PDF or final acknowledgement
  checkbox. The UI sends `ai-review-request-2026-10-29` for upload/submit actions;
  store that exact marker and server time in the backward-compatible processing field.
  It is distinct from legacy checkbox receipt `ai-review-2026-10-11`, which remains
  accepted and readable. Never fabricate a legacy checkbox acknowledgement.
  The only ordinary submission checkbox is public display after acceptance.
  File intake does not ask authors to distinguish papers from short notes; both
  use paper review. A prepared default-paper source consisting solely of open
  statements is treated as a conjecture. Explicit note inputs stay compatible.

Venue preparation accounts explicitly configured in `WISHPOOL_INTERNAL_SUBJECTS`
use CMA for their new initial manuscript and paper-problem readings, followed by
separate CMA source audits. They have no active-submission count cap. This grants
no roles, publication waiver or automatic formalization credit. Each round pins its
backend; absent historical pins remain Oracle, preserving live task identities.
API and worker use the same allowlist. Ordinary submissions retain the pipeline below.

Review stages:

| Code | Stage | Filed by |
|---|---|---|
| S0 | Source: the source is read, the PDF compiles, AI use is disclosed | worker (TeX Live) |
| S1 | Statements and their dependency graph | **the author** |
| S2 | Literature: no main result is already stated or directly implied | **Codex auditor** (checks the GPT Pro report against the source) |
| S3 | Escape: bind-only vs content, escape witnesses | **Codex auditor** (source-based correctness, shape and witnesses) |

Threshold (`api/crates/layer2-core/src/policy.rs`, the only authority):
accept on **escape_witness** (a main result is judged content and is not
known) or **open_problem_settlement** (a main result settles a named, sourced
open problem the literature had not settled). Conjectures are accepted for
**open_conjecture** iff at least one main open claim is audited well-posed,
`open`, and `content` (a proof would carry new mathematical content). Known true,
known false, special cases of known results, unclear status and bind-only readings
do not meet that rule. Existing human-stage and endorsement gates apply to every
kind. Every non-acceptance carries an author-facing reason.

For papers/notes, any explicitly audited `gap`, `error` or `not_checked` main
result blocks acceptance, even if a different result carries new content.
Unchecked main results settle as not accepted with a distinct reason, so the
automatic pipeline can still deliver its letter. Historical editor assessments
without correctness retain their existing meaning. Accepted inputs are not
retroactively rewritten. Public paper details include a count of confirmed
error reports bound to the published version and claims revision, omitted for
private records; the web page highlights nonzero counts without exposing audits.

Escape rates are reported only as exact readings on an explicitly built finite
arena. Never compute, estimate or display a percentage for a statement whose
arena was not built and measured.

New manuscript rounds pin review contract 4; paper-problem rounds retain contract 3.
Both use method `trureturing-content-2026-10-15`. Contract 4 supplies actual version-bound
manuscript origin to the initial reading and source audit. The metadata is untrusted;
reviewers must check attribution, source/version identity and scope before distinguishing
another artifact of the same contribution from independent prior mathematics. The work's
own theorem is not an independent prerequisite that makes its underlying argument
bind-only. Uncertain provenance stays insufficient; source declarations confer no
admission or verification waiver. Earlier rounds keep their original prompt bytes. Every audited statement carries a structured
prerequisite baseline, actual search scope, binding comparison, mathematical
delta, live witnesses, independent utility and limits. Proof escape may be
`undetermined`; missing evidence is not a bind-only finding. Correctness, proof
escape, information gain and utility remain distinct; no scalar score is used.
Content witnesses may be intermediate propositions or the conclusion itself.
Inline new wrappers and helpers, ignore unused terms, and stop comparison at
the stated existing prerequisites. Consumed bind-only helpers are legitimate
but confer no independent new content.

For contract 3 automatic admission, content requires a supported literature
comparison and category-appropriate mathematical utility. General theorems do
not need an artificial downstream consumer. Positive finite enumeration and
ordinary certified instances alone are supporting evidence. Checkers, numerical
reductions and counterexamples require an explicit problem, actual progress,
exact prerequisites and their discharge. A named open-problem settlement is a
separate route, even for bind-only proofs, but must audit the original statement,
hypotheses, prior status and matching proof/refutation against the confirmed
problem and reported opened source. Optional model/readout/task information-gain
readings do not decide admission. These are reviewer judgments, never compiled
Trureturing registrations, FIB proofs or Lean receipts. See
`docs/REVIEW_METHODOLOGY.md` for protocol and compatibility details.

New referee rounds additionally pin `knowledge_contract: 4`. Before Oracle
admission the worker runs an actual bounded source query for each confirmed
claim against one immutable, operator-admitted Trureturing publication/index.
Intent precedes execution; exact response bytes and receipt identities precede
model consumption. Oracle proposes structured per-claim reformulations (definitions,
hypotheses, quantifiers, conclusion and standard search terms); the system admits
and executes that batch before the audit. The auditor may request one additional
batch on the same pin, at most one query per claim; a fresh durable audit step
uses the saved results. Missing required reformulations or unexecuted retrieval
gaps keep the knowledge gate incomplete. Contract 3 pins `mathesis.source-ranker.identifier-paths.v1` in each
request digest: retain whole identifiers and case components, add a separate
module-path field, and rank reformulations by their search terms. The original
mathematical extraction is retained and hashed in the request scope. Candidate
ranking never establishes semantic equivalence. Contract 4 additionally budgets
the entire model-facing receipt, including declaration metadata, retaining bounded
source prefixes and a subset of returned declarations when a full catalogue will
not fit. Omitted declarations, excerpts and hits remain explicitly limited; exact
saved query responses and receipt identities are unchanged. Contract 3 retains
its original prompt projection, including on resumed work. Contract 2 preserves the original
full-field lexical reformulations and ranker. Contract 1
retains its original raw-query prompts and contract 0 has no retrospective
evidence. The same saved evidence reaches Oracle, the source auditor,
advice and the letter. Query completion is scoped; no match is not global novelty.
Source-audited coverage requires returned declaration identities, a joint premise
argument, parameter substitutions and discharged hypotheses. It is not a formal
coverage certificate. Old rounds without this contract retain their prompts and
have no retrospective receipts.

New-round admission requires both complete required queries and usable independent
comparisons. The application derives that gate from saved evidence, never a model's
claimed completion. Missing evidence leaves publication pending in review while
private advice and a letter can finish. Explicit mathematical errors still block
publication. Covered conclusions cannot support the new-result route; a named
open-problem settlement retains its separate evidence requirements. Staff may
restart a settled review on the same manuscript version after restoring the
knowledge source; previous rounds and queries stay immutable. Review-only requests
remain private and never receive a publication decision.

New rounds also pin `computation_contract: 1`. An attachment explicitly declaring
`replay_protocol: mathesis.rational-projection-amgm.v1` requests bounded exact
rational replay of a fixed three-projection matrix expression. The worker stores
its actual result against the immutable source/attachment, round and claims
revision before model admission. Retries reuse receipts; old contract-0 rounds
retain their prompts. Oracle, audit, advice, letter and private author reports
share the saved finite result. No uploaded code runs. Invalid inputs and checked
arithmetic overflow produce explicit non-results. A source audit must establish
correspondence to the manuscript; computation alone changes no policy or credit.
See `docs/COMPUTATION_REPLAY.md` for the input and limits.

Private research obligations are a separate author-authorized path after a source
audit. A current unresolved hypothesis or bridge retains its exact source, query
receipts and complete joint binding. The submitting author supplies the full goal,
authorizes 1–5 private proof attempts with an expiry within 30 days, and confirms
the independently elaborated and independently inspected target in a cookie session.
At most three target preparations are admitted, with one inspector-directed
regeneration each. The existing verifier checks attempts under source, target,
authorization and live-lease fences. Research histories are append-only in the
private referee aggregate; solutions/modules use GridFS. Private verification
receives no public proof credit and does not change correctness, coverage or
publication. An author can request a fresh source audit of a checked result;
its frozen trigger adds internal evidence only to newly admitted rounds.
See `docs/RESEARCH_OBLIGATIONS.md`.

## 2. Repository layout

The root contains exactly: `.github/ api/ web/ sdk/ infra/ docs/ CLAUDE.md
README.md .gitignore .dockerignore`. Tooling config lives inside `api/`,
`web/` or the SDK package it governs. `sdk/` holds self-contained client
packages (each with its own manifest and lockfile). CI rejects anything else.

## 3. Layers

```
api/crates/
  layer1-public/   Layer 1: REST projection of Layer 2. No business rules.
  layer2-core/     Layer 2: domain, threshold policy, judgements and analysis,
                   formalization, contribution network, ports, services. No I/O.
  layer3-latex/    Layer 3: unpack, read and compile LaTeX sources.
  layer3-review/   Layer 3: models via NyxID: Oracle broker and CMA.
  verifier/        stateless, credential-free Lean checker and binary.
  mathesis/        binary: composition, MongoDB (GridFS for files), NyxID
                   sign-in and delegated tokens, workers.
sdk/contribute/    the contributors' CLI and MCP server.
```

- Layer 1 depends only on Layer 2. Layer 2 depends on no workspace crate and
  no I/O crate. Layer 3 crates depend on neither Layer 1 nor Layer 2. CI
  enforces it.
- Authorization is decided in Layer 2, never in a route or the frontend.
- Every interface is a projection: the web app calls no route a third-party
  client could not call.
- Provider-specific code (a model API, a search engine, a TeX engine) lives in
  Layer 3; the contracts in `layer3-*/src/lib.rs` name none of the domain's
  rules.

## 4. The author owns the paper

- A paper in draft, in review, review complete, not accepted or withdrawn is visible only to
  its authors and to staff (editors, reviewer accounts, admins). Respond
  `not_found`, never `forbidden`, to anyone else.
- Contributors see a statement, its dependencies, and the paper's title and
  abstract only while the author keeps `open_to_contributors` on. Turning it
  off closes the paper's open tasks.
- Accepted mathematical details are public when the upload checkbox selected
  `public`, and may be made private later. Reviews and letters remain private
  under either setting. Formalization of a proved statement starts only after
  the author approves it. The private post-letter Lean probe exposes outcomes, never proof
  files, to the authors; publishing files still requires their agreement.
- Submitter and linked co-authors may read their private audit, the GPT Pro
  report and projected Lean outcomes through the referee view. Advice and probe
  files remain staff-only; public acceptance does not expose that private view.
- Only the author confirms statements (S1). A new version returns the paper
  to draft; a changed statement set voids S2/S3 reports, judgements and tasks
  of the old set by revision, never by deletion.

## 5. Judgements and credit

- Escape judgements retain provenance; never present them as Lean-checked.
  GPT Pro referees and Codex audits its report against the source. Layer 2
  files machine S2/S3 from that audit and automatically applies `Policy::decide`
  for publication submissions; review-only submissions complete without a decision.
  Only a proved main result audited `correct` can carry escape witnesses or
  ground a named open-problem settlement. Standalone conjecture admission uses
  the audited well-posed/open/content reading, with no proof-correctness claim. Recommendations are feedback, never decisions.
  The default human requirement is `{Claims}`; deployments may require human
  S2/S3 reports again. Editors are optional. Independent contributor judgements
  still corroborate by the existing different-family/account rule; editors may
  confirm them or resolve disputes.
- Judgements count when corroborated/confirmed, and literature checks/probes
  when accepted. Conjecture solving and paper formalization count only through
  the attempt → independent verifier path. No new `formalize` task, pull-request
  contribution, repository assignment, or editor-recorded proof is allowed.
  Historical documents remain readable with serde defaults and legacy variants.
- Conjecture credit: Others' conjectures (primary) and Total. A solver is an
  author when the person, or an agent's owner, is the submitter or a linked
  co-author of the source submission. Own conjectures count in Total. Paper
  proofs count under Formalized, confirmed reports under Errors found.
- Token usage sent through the API is stored as self-reported; only usage the
  NyxID gateway reported for hosted work is metered. The two are never summed.
- Delegated refresh tokens are stored sealed (AES-256-GCM) and never logged.

## 6. State

- MongoDB is the only authority; uploaded sources and PDFs live in GridFS.
  Aggregates carry `revision`; every replace is fenced on the revision read.
- Large referee aggregates use a revision-fenced Mongo header and a checksummed
  GridFS JSON snapshot. Legacy inline histories remain readable. Replaced
  snapshots are retained for offline collection, never deleted under readers.
- Review list status uses a small `review_status_v1` projection in the Mongo history
  header, atomically replaced with each history revision. Legacy reads derive it
  once from the checked history using a revision fence; no historical content,
  snapshot identity or aggregate revision changes. Authorization precedes this read.
- Stage reports are append-only. Accepted records are immutable.
- Machine work (compiling, S2 leads, S3 proposals) is a durable, leased job.
  A worker restart must not lose or duplicate admitted work.
- Normal worker shutdown drops local work and its heartbeat before releasing
  the leased job without spending a retry. Remote task identities are retained;
  failed release or process crashes recover through ordinary lease expiry.

## 7. Identity and security

- Sign-in is NyxID OIDC (authorization code + PKCE, nonce, browser-bound
  state). The browser holds only an opaque HttpOnly session cookie; its
  SHA-256 digest is stored. Machine clients present NyxID access tokens,
  verified locally against NyxID's JWKS.
- Cookie-authenticated unsafe requests must come from the public origin.
- Dev sign-in and memory storage exist for local work only and are refused on
  non-loopback binds.
- Uploaded LaTeX is untrusted. Archives are unpacked with limits on size,
  count and paths. TeX runs without shell escape, with paranoid
  `openin_any`/`openout_any`, a cleared environment (no deployment secret is
  visible to it), a fresh work directory and a deadline.
- No secret, token or credential enters the repository, including tests and
  fixtures. Test keys are generated at runtime. `infra/api/secret.yaml` names
  keys and never carries values.
- Model output is untrusted. It is sanitised in the binary's review mapping;
  anything that cannot be represented faithfully is dropped and named in the
  report summary. Never invent a witness, a citation or an identifier;
  named prior works include the DOI, arXiv id or URL the model states it opened.
  Those sources are reported, never venue-verified. External facts the model
  could not open remain `not_checkable`. Never send secrets, credentials, token
  files or deployment configuration to CMA; only bounded paper/review data.

## 8. infra never diverges from the code

A change that adds or renames a configuration key, port, health endpoint or
image input updates `infra/` in the same change. The API image serves two
roles: `WISHPOOL_ROLE=api` (stateless replicas) and `worker` (exactly one
replica owning the TeX cache volume). CI checks that every key read in
`config.rs` appears in `infra/api/configmap.yaml` or `secret.yaml`.

## 9. Gates

API: `cargo fmt --all --check`, `cargo clippy --all-targets --all-features --
-D warnings`, `cargo test --workspace` (with `WISHPOOL_TEST_MONGODB_URI` for
the MongoDB tests and `WISHPOOL_TEST_TEX_BIN` for the compile tests),
`cargo deny check`. SDK: the same fmt, clippy and test gates inside each
`sdk/*` package. Web: `npm ci`, `format:check`, `lint`, `typecheck`, `test`,
`build`, `npm audit --audit-level=high`. Cross-cutting: gitleaks, image
builds, kubeconform.

Pull requests run the gates of the areas they change; pushes to `develop` and
`main` run all gates.

The static showcase also passes `npm run build:showcase`. Real Lean tests are
explicitly ignored unless selected with `--ignored` and a prepared
`WISHPOOL_TEST_LEAN_WORKSPACE`; the verifier image build must run them and fail
on missing inputs. All verifier manifests are included in schema validation.

## 10. Branches

`develop` is the default branch; `main` is production and accepts PRs only
from `develop`. No force pushes to either.

## 11. Area contracts

The files under `docs/contract/` are part of this contract and equally
binding. Sections 1–10 above apply to every change. Before changing code in an
area, read its file; a rule that governs one area lives only there. Section
numbers are stable: add a new area as the next number, with its own file and
one row here.

| § | Area | Contract | Read before changing |
|---|---|---|---|
| §11 | Referee rounds and feedback | [`docs/contract/11-referee-rounds.md`](docs/contract/11-referee-rounds.md) | `api/crates/mathesis/src/review/`, `api/crates/layer3-review/`, `layer2-core/src/app/referee.rs`, `review*.rs`, CMA and Oracle configuration |
| §12 | Solving and the leaderboard | [`docs/contract/12-solving.md`](docs/contract/12-solving.md) | `api/crates/verifier/`, `layer2-core/src/app/solving.rs`, `lean_statements.rs`, `error_reports.rs`, `donations.rs`, leaderboard and profiles, prepared libraries (`docs/LIBRARY_BINDING.md`) |
| §13 | Research service intake and discovery | [`docs/contract/13-research-services.md`](docs/contract/13-research-services.md) | `layer2-core/src/app/deposits.rs`, `review_report.rs`, `finding_responses.rs`, `verification_bundle.rs`, archive search |
| §14 | Mathesis identity and source projects | [`docs/contract/14-source-projects.md`](docs/contract/14-source-projects.md) | `layer2-core/src/model/project.rs`, `attachment.rs`, Lean deposits, manuscript evidence, naming |
| §15 | Downstream knowledge consumption | [`docs/contract/15-knowledge.md`](docs/contract/15-knowledge.md) | `layer2-core/src/app/knowledge.rs`, `model/coverage*`, the knowledge importer |
| §16 | Consented binding discussions | [`docs/contract/16-binding-discussions.md`](docs/contract/16-binding-discussions.md) | `layer2-core/src/app/bindings.rs`, `model/binding.rs`, `web/src/pages/Bindings.tsx` |
| §17 | Authorized downstream feedback | [`docs/contract/17-authorized-feedback.md`](docs/contract/17-authorized-feedback.md) | `layer2-core/src/app/feedback.rs`, `model/feedback.rs`, `web/src/pages/Feedback.tsx` |
| §18 | Versioned downstream manuscript delivery | [`docs/contract/18-manuscript-delivery.md`](docs/contract/18-manuscript-delivery.md) | `layer2-core/src/app/manuscripts.rs`, `/manuscript-deliveries` |
| §19 | Academic submission interface | [`docs/contract/19-academic-interface.md`](docs/contract/19-academic-interface.md) | `web/src/pages/Submit.tsx`, `Papers.tsx`, author and staff manuscript views, `layer2-core/src/app/preparation.rs`, `people.rs` |
| §20 | Imported contribution history | [`docs/contract/20-contribution-history.md`](docs/contract/20-contribution-history.md) | manuscript `origin.contributions`, `layer2-core/src/model/provenance.rs` |
| §21 | Collected-paper reading guides | [`docs/contract/21-collected-reading-guides.md`](docs/contract/21-collected-reading-guides.md) | `layer2-core/src/app/explanations.rs`, collected explanation jobs |
| §22 | Collected sources and formalization participation | [`docs/contract/22-collected-sources.md`](docs/contract/22-collected-sources.md) | `layer2-core/src/app/collected.rs`, `problem_library.rs`, `proof_contributions.rs` |
| §23 | Collected open problems | [`docs/contract/23-collected-open-problems.md`](docs/contract/23-collected-open-problems.md) | `layer2-core/src/app/collected_problems.rs`, `model/collected_problem.rs`, `POST /collected-problems` |
| §24 | Result-centered knowledge reading | [`docs/contract/24-result-knowledge.md`](docs/contract/24-result-knowledge.md) | collected source indexes, `/collected-papers/{work}/knowledge` |
| §25 | Established theory library | [`docs/contract/25-theory-library.md`](docs/contract/25-theory-library.md) | `web/src/lib/theoryLibrary*`, subject and field placement |
| §26 | Dependency graph as the knowledge entry | [`docs/contract/26-dependency-graph.md`](docs/contract/26-dependency-graph.md) | `layer2-core/src/app/research_graph.rs`, `web/src/components/DependencyGraph.tsx` |
| §27 | Published collected-paper DOI | [`docs/contract/27-collected-doi.md`](docs/contract/27-collected-doi.md) | collected archive DOI sidecar and its registry adapter |
| §28 | Automatic collected-paper formalization | [`docs/contract/28-collected-formalization.md`](docs/contract/28-collected-formalization.md) | `layer2-core/src/app/collected_proofs.rs`, `mathesis/src/review/collected_proofs.rs`, `layer3-review/src/lean_environment.rs` |
| §29 | Visible formalization pool | [`docs/contract/29-formalization-pool.md`](docs/contract/29-formalization-pool.md) | `layer2-core/src/app/formalization_directory.rs`, `web/src/pages/Formalizations.tsx` |
| §30 | Published Lean evidence in the reader | [`docs/contract/30-published-lean-evidence.md`](docs/contract/30-published-lean-evidence.md) | paper, result and graph readers of collected Lean sources |
