# Mathesis

This file is the contract for anyone — human or agent — working in this
repository. Rules here are binding.

## 1. What Mathesis is

A venue where mathematicians submit their own papers, short notes and
conjectures through one entry and one review pipeline. The venue reads their
statements, audits the referee's mathematical reading, and gives accepted work
one shared `WP-<year>-NNNN` record sequence. Authors may instead choose an independent
private Orchestra review, which never applies publication policy or creates a record.
Short notes follow exactly the paper
pipeline; only their labels differ.

- **Input**: papers and notes upload `.tex`, `.zip` or `.tar.gz` sources.
  Conjectures use the same endpoint with either LaTeX source or a typed title,
  statement (at most 20,000 characters), optional background (50,000) and origin.
  Typed input becomes a minimal stored `.tex`, using the same reader, blob store,
  PDF build and author confirmation. Its single conjecture is a main claim.
- **Statements**: the author confirms kind, main or supporting role and
  dependencies. Papers/notes require a proved main result; conjectures require a
  main conjecture or question. Stored documents without `kind` load as `paper`.
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
- **Public layers**: the upload checkbox “Make public after acceptance” defaults
  to checked for all kinds. Publication happens at the decision, before advice,
  letter or Lean. The author may later change visibility through the existing
  route. Public details contain title, authors, abstract, DOI and record id;
  statements with dependencies; witness lemmas of audited-correct main content
  results; and verified Lean artifacts. Lean ✓ means a verifier-passed proof.
  Correctness labels, comments, rationale, audit summaries, referee reports,
  letters and advice stay private to authors/staff. Private or legacy undecided
  records expose title, authors, kind and record id only.
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
- **Consent**: every source upload, including revisions and typed conjectures,
  requires explicit, unchecked agreement to third-party AI processing (OpenAI
  ChatGPT via Oracle, Codex on CMA). Store version `ai-review-2026-10-11` and
  server time; missing or outdated consent is refused. Legacy records load
  without a fabricated consent receipt.


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

New referee and paper-problem rounds pin review contract 3 and method
`trureturing-content-2026-10-15`. Every audited statement carries a structured
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

The static showcase also passes `npm run build:showcase`. Real Lean tests are
explicitly ignored unless selected with `--ignored` and a prepared
`WISHPOOL_TEST_LEAN_WORKSPACE`; the verifier image build must run them and fail
on missing inputs. All verifier manifests are included in schema validation.

## 10. Branches

`develop` is the default branch; `main` is production and accepts PRs only
from `develop`. No force pushes to either.


## 11. Referee rounds and feedback

- S1 confirmation queues a referee round on the exact paper version and
  statement revision. Layer 2 owns ordering, input fencing, immutable settled
  steps, report filing, decision application, letter validation and view rules.
- The publication pipeline is GPT Pro referee → Codex audit → automatic escape-analysis
  decision → advice → automatically delivered letter → accepted-only Lean probe.
  Publication is immediate when the selected visibility is public. Paper/note
  probes and conjecture statement elaboration follow the delivered letter.
  No step requires a human editor. Editors/admins may intervene, restart settled
  reviews while the paper is in review, or send additional letters. Staff can also
  restart a settled, failed current private review after review completion. This
  records a new operational re-review round without changing the source, prior
  readings/letters, visibility or publication state; successful private reviews
  and live rounds cannot use this recovery path.
- For conjectures, the referee supplies well-posedness reasons, open/known/unclear
  status, reported work names, content/bind-only escape reasons and sharpening
  suggestions. CMA Codex checks missing symbols/quantifiers and small cases, and
  may search literature and open sources. Known-result matches require a
  matching argument and the reported opened source, or reproducible local
  mathematical evidence. Unopened external knowledge becomes unclear.
- The audit checks source, report JSON and full text, computing where useful.
  It covers every confirmed statement with correctness, a one-sentence comment,
  proof shape/witnesses where applicable, and agreement with the referee.
  External facts the agent could not open are `not_checkable`. Every named
  prior work includes its reported opened source; never invent citations.
- Layer 2 appends audit-derived S2 (known prior works) and S3 (main-result escape
  assessments) under the statement revision fence. Gap/error/not-checked main
  results have no escape witnesses and cannot ground acceptance. Publication is
  decided only by `Policy::decide`, never either model's recommendation.
  Report and decision replay is idempotent, including recovery after record
  insertion. Accepted records and their publication inputs remain immutable.
- Advice runs after the applied decision (or private review completion) for every audited paper and feeds
  improvement suggestions to the letter. Formalization candidates are used only
  after acceptance. A sketch does not verify a theorem.
- A completed letter step atomically delivers the letter in-app, with its round,
  audited assessment, the auditor as sender, and `edited=false`. It states the
  applied decision and record/reasons first, then audited feedback, confirmed
  changes, "please check" for unavailable facts, and improvement suggestions.
  It contains no Lean results. Private review letters state completion and feedback,
  never acceptance/rejection. Manual additional letters remain available.
- Lean runs only after a delivered letter and acceptance, using advice's proved
  candidates. The binary compiles each file and checks axioms. Authors see
  statement, outcome and theorem name afterwards; no second letter is needed.
  A compiled probe is not a recorded formalization; publishing files needs the
  author's agreement.
- Author-safe rounds include payload-free stage progress: state, recorded times and
  retryability, including hidden advice and letter stages. Results, free-form errors
  and provider metadata never enter this progress projection. Only an actual delivered
  letter establishes delivery; queued author replies remain visibly awaiting re-review.
- Staff receive full review history. Submitter and linked co-authors receive
  reports, audits, sent letters and projected Lean outcomes through the existing
  referee GET; advice, drafts, provider metadata and Lean files remain private.
  Anyone else receives `not_found`, including readers of an accepted paper.
- Referee readings remain machine judgements under `wishpool:referee` by default
  (engine `nyxid-oracle`). Audit reports and automatic letters use the separate
  `wishpool:auditor` account by default (engine `nyxid-cma`; `codex-cli` only for explicit development fallback). All model output is
  untrusted and sanitised in the binary; never invent witnesses or citations.
- A referee request goes to every configured Oracle pool; the first completed
  answer is the report. The default pool is `chrono-chatgpt-pro-pool` on the
  standalone Oracle broker, reached through NyxID's proxy service `oracle`
  (`nyxid proxy request oracle api/v1/oracle/...` for the local CLI).
- The NyxID Oracle task and submission identity are durable; the worker releases
  its lease between polls without spending retries and renews a running job's
  30-minute lease about every five minutes. A lost lease fences the pending
  result. Transport completion is not mathematical verification. Model output
  is sanitised in the binary.
- All Codex agent steps use CMA through NyxID: audit, advice, letter,
  accepted-paper Lean probe, conjecture target generation and paper-problem
  audits. CMA network access is on. Paper TeX and needed inputs travel inline,
  split into turns above 200 KiB. Final messages are JSON; Lean entries carry
  every file's complete text. Answers are capped at 2,000,000 bytes.
- CMA agent/response ids, chunk progress, deadlines and terminal answers are
  durable private steps in the referee aggregate. Stable keys bind submission,
  version, statement revision, round, step and attempt. Worker restarts resume
  polling with those ids; uncertain mutations reuse the original key and body.
  Agent steps have at most 3,600 seconds. The lease heartbeat stays active.
  New CMA runs pin delivery contract 1: the final input chunk repeats bounded
  audit/advice/letter instructions and the exact response-format requirement without removing
  input bytes. Legacy saved runs retain their original framing and keys. Only a
  terminal `agent_command_queue_timeout` with explicitly empty output may retry
  its chunk once on the same agent, after saving the original response receipt,
  with a distinct durable key and within the original deadline. Live work,
  ambiguous failures and executed output are never restarted by this path.
- A worker has one compile lane, one verification lane and 1–8 remote review
  lanes (`WISHPOOL_REVIEW_CONCURRENCY`, default 2). Compile jobs never wait on
  CMA cleanup. Each job retains its own lease and cancellation fence.
- Every CMA configuration pins a published agent profile. Before a new Oracle
  request is admitted, read-only CMA resource preflight checks the eligible
  author/operator resources without creating an agent or recording a payer.
  Execution admission still chooses and persists the payer in Layer 2; existing
  attempts never change identity during preflight. Resource readiness is not
  proof of working model credentials or quota.
- CMA output is untrusted: the binary writes returned Lean text into fresh
  scratch and checks it locally; the credential-free solution verifier is
  unchanged. Always stop and delete the CMA Agent after the final answer or on
  cancellation/failure, best effort with logged failures. Credentials stay in
  the transport, never in a model prompt.
- `WISHPOOL_AGENT_BACKEND=cma|codex-local` selects the carrier; configuring a
  CMA workspace defaults to CMA. `codex-local` is an explicit development
  fallback, refused on non-loopback binds. Oracle CLI and CMA CLI are also
  loopback-only. CLI children inherit only operator `PATH` and `HOME`.
  The local fallback uses fresh source/scratch copies and disables its sandbox
  network. Production CMA HTTP through NyxID accepts an operator API key scoped
  to CMA, or a user access token from the existing sealed refresh path or a mounted
  token file. NyxID/CMA enforce current authority; credential prefixes are not
  authorization. Mounted user access tokens do not refresh automatically.
  Every new configuration key must be declared in infra (§8).

- Author CMA connections use ordinary NyxID `offline_access` consent, without
  resource or service restrictions, in separate sealed `cma_review` generation
  slots. `WISHPOOL_CMA_BYOK_ENABLED` defaults false; enabling it requires NyxID,
  HTTP CMA transport, `WISHPOOL_TOKEN_KEY` and `WISHPOOL_CMA_PUBLIC_URL`.
  Authors choose an existing ready workspace owned by their NyxID subject in
  a Sandbox whose pinned profile uses `own_credentials`. Automatic provisioning
  is a follow-up. Connecting does not require hosted donations.
- Before any CMA I/O, Layer 2 durably admits the submitting author's ready
  binding for eligible submission steps or the operator default. Co-authors
  are never selected as payers; supporter attribution is separate from account custody. Replays retain the attempt's payer, generation and workspace.
  A terminal author account/auth/setup/credential/quota refusal, or an agent
  failure labelled `cause_unknown`, admits at most one recorded operator
  successor with fresh request keys. Transport errors, 429/5xx, uncertain writes,
  mathematical/JSON errors and local verifier failures never switch payer.
  Cleanup uses each attempt's original account. Private referee views expose
  only the step, ordinal, payer, sanitized reason, status and times, before letters.

Author CMA execution requires `WISHPOOL_CMA_AGENT_PROFILE` with an exact
`{"id":"agp_…","revision":1}` pin to a published agent profile readable by
both the operator and author accounts. The exact pin is retained in each
creation intent for retries. Workspace facts are fetched and decoded in Layer 3;
Layer 2 checks ownership, readiness and `own_credentials` before saving the
binding under its source revision. Refresh throttling, outages, ambiguous
refusals and concurrent token rotation remain transient, with the same payer.
Disconnect/reconnect revokes the old generation; outstanding agent cleanup is
best effort and may fail after revocation. Cleanup never substitutes operator
credentials for author resources.

CMA payer mapping (Layer 2 decides before provider I/O):

| Step | Payer and attribution |
|---|---|
| Referee audit, advice, letter, accepted-paper Lean probe | Ready submitter binding; otherwise operator |
| Standalone conjecture target, admitted paper-problem target, author-approved proved paper-statement target | Ready submitter binding; otherwise operator |
| Target faithfulness audit, including regeneration | Ready submitter binding; otherwise operator |
| Independent paper open-problem audit | Ready submitter binding; otherwise operator |
| Error-report text audit | Operator, never the author or reporter |
| Venue-agent attempt on a confirmed target | Operator account with phase-D donation attribution, never the author |
| Existing legacy CMA run | Original operator account; never retroactively charged to an author |

Operator-funded target generation, faithfulness audits, Lean probes and venue
attempts retain phase-D supporter attribution, including an operator successor.
Author-funded steps create no donation attribution. Attribution alone charges no
tokens: only a verified NyxID usage receipt can meter usage. Replays retain the
saved payer, workspace, generation and request identity. Oracle remains operator
funded; local elaboration and proof verification carry no CMA payer.

## 12. Solving and the leaderboard

- Only a verifier-passed Lean proof or disproof of the author-confirmed target
  counts. The API uses the Layer-3 `Verifier` port and never accepts a client's
  or model's claim of success. Solutions import `Target` and declare exactly one
  `wishpool_solution : wishpool_target_prop` or
  `wishpool_disproof : ¬ wishpool_target_prop`; auxiliary lemmas and definitions
  may precede it.
- The verifier builds Target and Solution as separate modules in a fresh copy
  of the pinned Lean workspace, then independently checks the imported constant
  against `_root_.wishpool_target_prop` and prints its axioms. Only `propext`,
  `Classical.choice`, and `Quot.sound` are allowed. Reject axioms, constants,
  opaque/unsafe declarations, sorry/admit, implementation overrides, extern,
  macros/notation/syntax/elaborators/initializers, weakened options, executable
  commands, and imports other than Mathlib, Target and submitted supporting
  modules validated under §14.
- The verifier is stateless and credential-free: cleared environment (PATH and
  fresh HOME only during compilation), fresh work directory, bounded input and
  output, and a deadline (default 1200 seconds, maximum 3600). Local workers use
  a subprocess. Infrastructure uses a separate Deployment with no mounted
  secrets or service-account token and a NetworkPolicy denying all egress.
- Attempts are durable leased jobs, private to their owner/staff until verified,
  limited per entrant and conjecture, and idempotent by entrant, target digest,
  and solution digest. A revision-fenced write records the verifier receipt.
  The earliest verifier timestamp wins; an attempt id breaks an exact timestamp
  tie. Later checked attempts are public as “also verified” and earn no points.
  The author receives an in-app notification. Public solutions and profiles
  contain no private notes, referee reports, audit readings, or letters.
- One leaderboard contains people and pure agents. “Others' conjectures” is
  proved plus disproved conjectures whose source submission involves neither
  the entrant nor its agent owner as submitting author or linked co-author.
  “Total” includes own conjectures. Sort others descending, total descending,
  earlier last solve, then entrant id. Filters select all/people/agents and
  all time/current UTC month. Profiles show both solved counts, Formalized,
  Errors found and the corresponding winning proof receipts.
- Author-approved proved paper statements use the same target generation,
  faithfulness audit, cookie-session author confirmation, attempt and verifier
  path as conjectures. Their verified proofs display Lean ✓ with solver credit
  and count under Formalized, never on the conjecture leaderboard.
- Before confirmation, a separate CMA Codex audit compares the original statement
  and exact target. Its verdict is faithful/weaker/stronger/different/trivial,
  with one paragraph and the specific mismatch. Missing/invalid audits never
  unlock confirmation. A non-faithful result regenerates once with that reason;
  the second check is shown even if still non-faithful. Old unaudited targets
  return to preparation. Public pages say “proved the author-confirmed Lean target”.
- Signed-in people and their agents may report errors in accepted public
  statements, targets or reviews, with text ≤20,000 characters and/or a Lean
  disproof. Reports are limited to five per entrant per rolling 24 hours across
  all items, with revision-fenced admission. Lean evidence enters the ordinary
  verifier immediately on the report job; text enters a durable CMA audit with
  the same evidence sanitization. Verdicts are confirmed/refuted/unclear.
  Reports stay reporter/author/staff-only until confirmed; confirmed text and
  verdict are public while the source is public. Refuted/unclear reports stay
  private. Confirmation credits the reporter and notifies the authors in-app.
- Every confirmed error report and verifier-passed disproof of a statement
  of an accepted paper queues a fresh referee round automatically. Trigger ids are
  durable and idempotent; rounds snapshot them and recheck the evidence. An
  active round finishes before the next starts. Accepted publication inputs
  remain immutable; the new private audit and assessment are retained.
  Confirmed target reports also void the exact digest and regenerate it.
- Donations fund our agents, never impersonate donors as solvers. Venue agent
  id `wishpool-agent` is owned by `wishpool:venue`; both `wishpool-agent` and
  `mathesis-agent` names are reserved. A confirmed
  open target queues one durable CMA attempt per digest; Mathesis runs Lean.
  Supporters is alphabetical and unranked. Run attribution is durable, separate
  from usage. Only a NyxID accounting receipt can charge tokens, atomically and
  once per run. CMA v2 rejects restricted delegated tokens: use the configured
  venue token, retain attribution, and show zero until a NyxID usage receipt is
  available. CMA/model/client token estimates never become metered usage.
- Public accepted items say “Reviewed by GPT Pro and audited by Codex”.

## 13. Research service intake and discovery

- The archive has MongoDB text/metadata indexes. Filtered search uses immutable
  publication inputs and rechecks live visibility before returning results.
  Search must never match private drafts or private review text.
- `/submissions/{id}/review` exports a version/digest-bound Orchestra dossier
  using the author-safe referee projection, even for staff. Authors can see
  available progress, reports and audits before the delivered letter.
- `/formalizations` receives existing Lean modules as private, attributed
  deposits, with original authors, source reference, pinned toolchain and license.
  Sources live in GridFS; metadata lives in `lean_deposits`; exact retries are
  idempotent. Receiving a source is never proof verification or paper acceptance.
  Concurrent retries discard only their own fresh source blobs after a confirmed
  conflicting deposit is read, preserving every blob it references. Cleanup is
  best effort and never runs after an uncertain insert outcome.
- Deposit owners may explicitly request verification against an existing public,
  author-confirmed target. Reuse the ordinary attempt and independent verifier;
  link attempts under a revision fence. No separate acceptance route, arbitrary
  project execution or editor verification is introduced.
- User-facing services are Orchestra review, deep reasoning and formalization.
  CMA account configuration belongs in advanced compute settings. NyxID remains
  the identity and connection provider. API, OpenAPI, CLI and MCP project the
  same Layer 2 services. See `docs/RESEARCH_SERVICES.md` for exact current scope.

- Audited concerns retain reported location, evidence and basis separately, including
  offline calculations. A reported source is not an authenticated citation.
- Authors/co-authors may append immutable responses to an audited finding, bound to
  the current confirmed source version and claims revision. The ordinary durable
  referee queue rechecks them. Exact retries are idempotent; at most five responses
  per paper per 24 hours, 200 total. Replies never resolve findings automatically.
  Queue reconciliation recovers a saved response after enqueue failure.
- Review JSON schema 2 and `/review.md` include each round's source digest and author
  responses. Both use the author-safe view, including when requested by staff.
- New rounds pin review contract 3. Contract 1 introduced re-review assessments;
  contract 2 additionally carries bounded attachment contents and their actual
  version-bound hashes into audit, advice and letter prompts. Earlier rounds
  retain their original prompts, including when evidence was not transported.
  Contract 3 adds the structured Trureturing methodology above. Historical
  reports and decisions retain their original meaning; no automatic backfill.
  Re-review audits assess each admitted author
  response as addressed, unresolved or not_checkable, with a concrete explanation.
  Attachment references resolve to paths and actual SHA-256 digests in that round's
  source version. Unknown/duplicate response outputs or unusable evidence cannot
  establish addressed. Missing model conclusions become explicitly not_checkable.
  These are reviewer judgments, never execution or Lean receipts; old findings stay
  immutable. Legacy rounds retain their admitted prompt bytes and receive no fabricated
  retrospective conclusions.
- Public verified attempts offer a JSON verification bundle: exact confirmed target,
  solution/modules, pinned toolchain, attribution, source/claim identity and receipt.
  Current public visibility and target validity are checked on export. Rechecking
  requires a prepared matching Lean/Mathlib workspace; no historical checker binary
  revision is invented and no private probe files are exported.

## 14. Mathesis identity and source projects

- Mathesis is the public product and repository name. Follow
  `docs/MATHESIS_MIGRATION.md` for compatibility: persistent WP ids, proof names,
  identity subjects, database names, cookies and WISHPOOL server config remain.
- Formalization is a primary participation path. The public directory lists
  only current confirmed public targets and independent verifier results;
  private deposits, queued/rejected attempts and stale targets are excluded.
- Deposits may include 512 dotted-path supporting Lean modules, at most 8 MiB together
  with the optional Solution adapter (1 MiB maximum). Inert project configuration
  is preserved separately (16 allowlisted root files, 1 MiB total). They are immutable GridFS sources. Record original authors,
  formalizers and optional full Git revision separately from uploader identity.
- Project verification uses the same independent checker and durable attempts.
  Scan every module, reject cycles, unsafe constructs and reserved/shadowing
  names, compile in dependency order, and bind all sources into the receipt.
  Do not execute user Lake scripts or install custom dependencies. Only the
  existing pinned workspace supplies dependencies. Single-file hashes stay stable.

- Complete projects can be source-only deposits. Preflight reports missing imports,
  cycles, dependency/target pin conflicts, and restricted-profile constructs.
  Verification may attach a separate immutable Solution adapter; saved modules stay intact.
  Supporting modules may contain namespace/section blocks and Mathlib submodule imports;
  each Mathlib import must resolve to a prepared artifact before Lean runs. Root target
  and adapter scans remain strict, including comments. Preflight is not a receipt.
- Manuscript archives may declare `mathesis-evidence.json` schema 1: up to 100
  relative-path attachments with kind, description, optional claims and full Git revision.
  Derive byte counts and SHA-256 from actual archive files; bind inventory to PaperVersion.
  Source access rules also protect attachment downloads. No attachment is a proof receipt.
  Review export includes inventories by version. Oracle gets up to 64 KiB per UTF-8
  attachment / 256 KiB total; CMA gets a bounded source inventory and up to 512 KiB per
  UTF-8 file / 2 MiB total. Explicitly identify every inventory-only file; never claim
  it was read or validated. Uploaded scripts are not automatically executed.

- See `docs/REVIEW_FOLLOWUP.md` for version-bound response assessments and asynchronous
  CMA cleanup. A stop acknowledgment does not imply that an agent can yet be deleted.

- Keep legacy review prompt bytes unchanged when no computational evidence is declared;
  existing admitted task input digests must remain resumable across this upgrade.

## 15. Downstream knowledge consumption

- Mathesis consumes independently admitted, existing Trureturing publications;
  ingestion never executes upstream code or Lean. Preserve original report
  identities, source license and frozen/proven-not-yet-frozen status. A module's
  imports and frozen prerequisites are not declaration proof dependencies.
- Public knowledge indexes and original artifacts live in MongoDB/GridFS.
  Index revisions bind checked source and catalog bytes. Imports are immutable;
  partial source coverage, size limits and result truncation remain explicit.
  A diagnostic query is not a review receipt, and similarity is not entailment.
- Private knowledge query admissions, actual completions and comparisons append
  to the revision-fenced referee history. Keep source/claim/round/snapshot pins,
  complete joint premise sets, hypotheses and unresolved obligations. Ordinary
  Lean verification cannot qualify a claim as a recognized binding certificate.
- The importer CLI uses an operator's independent admission evidence; artifact
  trust flags and the presence of a CI URL are not authentication. See
  `docs/KNOWLEDGE_OPERATIONS.md` for input limits and the current integration scope.

## 16. Consented binding discussions

- Only accepted public publication submissions with contributor access enabled
  may open a public binding summary. The submitting author separately approves
  the exact statement, bounded context and selected actual-query declaration
  identities under an explicit summary license. Query text, manuscript evidence
  and private audit notes are not authorized for public disclosure by that act.
- Signed-in owners submit licensed structured joint bindings and challenges.
  Authors/co-authors cannot claim discovery credit on their own submission. A
  Mongo admission fence limits contribution/challenge requests to five per
  person per rolling day across all discussions; exact retries are idempotent.
- A durable operator-funded independent CMA inspection consumes the original
  manuscript and checksummed saved required source excerpts. Missing/truncated
  required evidence cannot support a binding. Only composition can persist the
  result, under source, consent, history and job-lease fences. No interface
  accepts a successful audit verdict from a contributor.
- Challenges and appeals append evidence and receive independent inspection.
  Effective credit may revoke/restore; earlier events stay immutable. Credit
  belongs to the first independently supported equivalent/instance/implies
  binding, once per canonical source/target/premise/relation/correspondence set.
  Proof use and refutation keep their distinct meaning. This is no proof receipt,
  recognized coverage certificate, novelty upgrade or publication decision.
- Public discussions/profiles expose only approved content, licensed contributor
  inputs and controlled audit verdicts. Detailed audit evidence is private to
  authors/staff. Revocation, private visibility, contributor closure or source
  changes stop new public access and live managed work; previously issued public
  licenses and private history persist. Upstream export authorization is separate.
- See `docs/BINDING_CONTRIBUTIONS.md` for bounds, routes and CLI/MCP parity.

## 17. Authorized downstream feedback

- Feedback proposals are immutable previews from six current public evidence
  channels: admitted open problem, verified solution, supported binding, public
  content witness, proposed four-slot DTR registration, and confirmed erratum.
  They pin source/version/claims, producer, attribution and exact payload digest.
  No preparation, public discussion or ordinary proof receipt grants export rights.
- Every required owner separately asserts applicable rights and approves the
  exact payload under Apache-2.0. Submitter consent never substitutes for a
  solution owner, binding contributor or reporter. Original attribution/licenses
  remain attached. Preparation, approvals, issuance, revocation and observations
  append to referee history; large payloads and evidence use checked GridFS blobs.
- Export rechecks all signatures and current evidence. Revocation/source changes
  stop new access, export and observation work; authors retain private history
  and previously issued licenses remain effective. Concurrent retries use the
  same package identity. The existing offline collector retains referenced blobs.
- A durable leased readback job only reads fixed public GitHub PR/commit endpoints
  and an independently admitted immutable source index. Merge metadata, exact
  ancestry/tree, full declaration identity, module provenance marker and original
  freeze status are separate observations. No HTTP/CLI endpoint accepts success
  receipts. Missing/rate-limited evidence is an explicit non-result, retained for
  inspection; a caller can request a later observation under a new key.
- Mathesis makes no upstream writes and claims no automatic mathematical
  correspondence, upstream admission, registration, freeze, or coverage certificate.
  See `docs/AUTHORIZED_FEEDBACK.md` for routes, limits and validation boundaries.
