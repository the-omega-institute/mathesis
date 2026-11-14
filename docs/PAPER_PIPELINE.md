# Paper content and shared formalization pipeline

This is the downstream integration plan. The existing publisher collects and
publishes eligible public papers without another model review. The material
adapter, web readers and local inspection commands implement the interfaces
below. Automated content authorship and upstream-result reconciliation remain
separate work: a plan or request is not a completed output.

## What readers see

Use the full paper title in lists, README headings, citation metadata and GitHub
repository descriptions. Keep the DOI-based repository address stable; authors
and readers should not have to read its identifier to recognize a paper. Papers
without a DOI retain their durable mapping when a DOI is later assigned. Titles
can change without moving repositories or breaking links.

A paper page offers the original paper and citation, bilingual readable story,
slides, video and appropriate interactive illustrations. The story assumes school
algebra and no university mathematics: introduce the question with concrete
objects, explain specialist ideas before naming them, and preserve the result's
actual assumptions and limits. Show only built
materials of that exact public version. Academic slides are currently Beamer
PDF plus source; editable PowerPoint is not an implemented export. Short
plain-language sidecars and the story are the same explanatory product, not
separate short and long generation tasks. Reuse a faithful complete exact-version
sidecar; expand it in the paper's authored editions only where necessary for
readability or missing explanation, without requesting a duplicate story job.
When no sidecar exists, author the same explanation directly from the frozen
manuscript. Preserve authorship and credit in derived materials. Video is a
required planned content product; its absence stays pending and does not block
publication or display of an otherwise eligible public paper and its existing
materials.

## Ownership

| Component | Responsibility |
|---|---|
| Mathesis | Public paper/statement identity, visibility, independent receipts, participation and platform credit |
| A paper repository | Frozen manuscript/citation, bilingual story and media sources, statement-to-theory mapping |
| Published books template | Renderer, visual/media contracts, build commands and caches |
| `mathesis-papers/catalog` | Public inventory, stable repository mapping, deterministic publication and local work plans |
| `trureturing-requests` | Shared, deduplicated requests, claims of work and progress discussions |
| `mathesis-papers/trureturing` | Optional workspace for a named collaboration; no mandatory staging merge or permanent sync/CI service |
| Mathlib / trureturing | Pinned formal mathematics, declarations and published validation evidence |
| `mathesis-papers/formalization` | Public projection of platform targets, receipts and credit; not a second competing proof queue |

## Content production

1. Freeze the public paper version, manuscript hashes, authors, citation and
   statement inventory. Private reviews, tokens and deployment configuration are
   excluded from content inputs.
2. Reuse an authored book if present. Otherwise create it using the installed
   Mathesis adapter. Read the original manuscript and public explanation; do not
   substitute an abstract for a self-contained story.
3. Establish the question, result, mechanism, examples, significance and precise
   limits in the two languages. Review mathematical correspondence and sources.
4. Derive figures, interactive examples, seminar slides and narrated video from
   that story through the installed upstream skills and tools. Commit sources;
   keep rendered media and run records outside the committed paper tree.
5. Run the complete paper/template checks and relevant builds. Video acceptance
   requires an actual playable MP4, not a successful build that skipped films.
   Review captions/narration, readability and mathematical scope.
6. Publish checked artifacts, then discover their exact-version manifest and
   read back links/files. Main-site material discovery must also be enabled;
   successful GitHub Pages publication alone does not update the API deployment.

The mechanical publisher does not invoke models. Inspect existing content before
admitting any CMA task. Content units are per paper/version/source fingerprint;
media also depends on the story source. Retry only missing/failed units. A
provider failure is an operational retry, never a publication rejection or a
claim that the mathematics is open. One credential-free local inspection is:

```sh
python3 scripts/plan_content.py --repository /path/to/paper-repo \
  --site /path/to/built-site --output /outside/paper-repo/content-plan.json
```

This command belongs to `mathesis-papers/catalog`. It checks frozen file hashes,
version/repository identity and advertised built formats. Its deterministic
unit keys are handoff inputs for a future runner; it does not call CMA, execute
uploaded manuscript code or make semantic mathematical judgments. `built` means
content output exists, never that a proof is independently verified. Template
changes and formalization-pin changes require their own revalidation even when
no content generation is needed.

## One shared formalization queue

Search pinned Mathlib/trureturing declarations and inspect their real scope
before preparing a request. Reuse an exact existing declaration when possible.
For a missing formalization of an existing written proof, follow the published
books-template registration contract: one mathematical result per issue, search
open and closed requests, register each consuming paper under the same issue,
and avoid separate requests for translations. Mathematical equivalence needs
review; titles, lexical matches and issue numbers are insufficient.

Expose the public shared queue from `/formalizations`, and link each paper's
registered requests in its materials. Mathesis is the intended public browsing,
claim-of-work and submission interface: a person or agent may participate through
the webpage or API, without being required to leave the site and claim a GitHub
issue manually. Both entry points must bind to the same canonical request and
exact confirmed target, not create competing work queues. Confirmed Mathesis
targets retain their existing independent verifier and credit rules. A contribution
made only on GitHub is attributed upstream but receives no invented platform
points. No change here bypasses an external author's existing target consent.

Keep these states distinct:

- Exact existing Lean coverage, with declaration and pinned revision.
- Written proof awaiting formalization, with a shared request.
- Partial coverage, with the precise uncovered clauses.
- A genuine unsolved conjecture/question.
- Unknown or unreviewed statement correspondence, with mapping still pending.
- Attempt/environment failure, recorded separately from mathematical status.

Issue open/closed status is live workflow context, not verification. When a
result is published/frozen upstream, downstream must inspect its statement,
parameters and hypotheses, bind it to each exact paper claim, update the pin
and all language editions, validate and rebuild. Only receipt-backed mappings
change the verified graph/coverage or platform leaderboard. The automatic
upstream-result-to-platform binding worker is not implemented by material links.

### Request submission and fulfillment adapter

The existing webpage and API share statement-scoped proof submission and
independent verification. They do **not** yet provide canonical request-to-target
routing, GitHub PR submission or automatic upstream-result reconciliation. The
shared requests repository currently provides issues, not a submission service.
The following is the downstream adapter design, not a claim of deployed behavior.

1. Bind the canonical request URL and scoped clauses to exact paper versions,
   confirmed target digests and authenticated contributor/agent-owner identities.
   Preserve the original mathematical authors and existing-library formalizers.
   A claim of work records public scope and progress, without an exclusive lock.
2. If a contributor supplies Lean code, send it through the existing bounded
   independent verifier. Reuse an exact saved receipt on a retry. No model call
   is needed to package known files, poll CI or repeat an identical check.
3. A normal durable background adapter prepares a focused contributor branch/PR
   through upstream's existing contribution route, retaining source attribution,
   contributor identity, exact files and full request URL. It tracks the existing
   upstream checks, review, admission and freeze. Retries reconcile saved branch,
   PR and commit identities rather than opening duplicates. It requires its own
   narrowly scoped GitHub write credential; no model receives that credential.
   It neither changes upstream CI nor adds a mandatory Mathesis-fork gate.
4. Use CMA only when mathematical work is needed: translation, proof construction
   or a substantive repository adaptation. A version/target mismatch requests
   adaptation or confirmation; an outage leaves work pending and retains handles.
5. Reconcile an admitted frozen declaration against each exact target and every
   requested clause. A verified proof, an upstream merge and complete request
   fulfillment are separate observations. Partial completion records covered and
   uncovered clauses and keeps the request open. Close only on full scoped
   fulfillment; preserve the issue, original authors, contributors, PRs and evidence.
6. Refresh the existing paper coverage, dependency graph and receipt projections.
   A change to the paper or target requires a new exact mapping, not a model redo
   of unrelated content. Existing declarations are reused before new proof work.

Show readers concise states: awaiting proof, checking, independently verified,
upstream admission pending, and admitted, with partial scope explicit where
applicable. Keep genuine open mathematical questions separate from established
results awaiting Lean formalization. GitHub issue closure alone never fills a
verified graph node or awards platform points.

Competitive rankings exclude Mathesis's venue agent/owner, configured internal
preparation accounts and their agents, and authors/their agents formalizing their
own papers. External contributors to our papers remain eligible. All checked
proofs retain attribution and receipts, subject to the existing public/version
fences; ranking exclusions do not transfer first-proof credit to a later repeat.
Author identity uses submitting or linked co-author accounts, not name guesses.

## Community contributions directly upstream

For contributors using GitHub directly, the normal route is a focused branch in
their own contributor fork and a PR to `the-omega-institute/trureturing:dev`. An upstream
branch is an alternative only when upstream permits that contributor to use it;
direct upstream submission is not a maintainer-only exception. Follow upstream
[contribution rules](https://github.com/the-omega-institute/trureturing/blob/dev/docs/CONTRIBUTING.md)
and the rules at the actual checkout revision, including library reuse, proof
admission, required CI and independent review.

1. Open the shared request and inspect its exact statement and current work.
   Comment with the scope you will tackle and link the working branch/PR. The
   comment is the public claim of work; no assignment bot or exclusive lock is
   implied. Coordinate overlapping efforts in that same issue.
2. Develop a focused change in a contributor fork, retaining the original
   mathematical authors and each contributor's attribution. Run the applicable
   development checks and submit that branch directly to upstream `dev`.
   There is no prerequisite merge into a Mathesis organization fork.
3. Meet upstream's current admission, independent review and required checks.
   Contributor/fork CI is development evidence; upstream acceptance and frozen
   declaration membership establish the shared-library publication boundary.
   Neither CI success nor merge alone proves correspondence to a paper claim.
   Keep full request URLs in PRs; a bare `Closes #N` can close a different repo's
   issue. Close the request only when its full stated scope is complete.
4. Bind each published declaration to the exact paper statement, hypotheses and
   version; refresh pinned editions and evidence. A partial theorem leaves the
   uncovered clauses explicit, even when a PR has merged.

The existing [Mathesis fork](https://github.com/mathesis-papers/trureturing) is
optional reversible preparation, separate from the generated
`mathesis-papers/formalization` projection. Use it only for a concrete
collaboration with a named owner, scope and end condition. It adds no second
queue, proof authority or compulsory integration gate. Inspect the actual
repository/network settings, workflow events, permissions, secrets, protections
and cache access before using it; fork behavior depends on those settings.
Creation or enabling workflows does not establish a successful proof-CI run or
upstream release authority. No ongoing mirror, scheduled synchronization or
extra CI service is funded by this plan. Preserve active branches if a later
collaboration needs an upstream refresh.

### Route decision and cost

| Option | Maintenance and participation | Decision |
|---|---|---|
| Mandatory Mathesis-fork integration before upstream | Two integration/review paths, extra CI and synchronization ownership; every public contributor waits on another gate without changing upstream authority | Reject for the present need |
| Direct upstream PRs, with an optional Mathesis collaboration fork | Ordinary contributors use the existing upstream route; a named collaboration pays for only its actual shared branch/checks | Retain as an on-demand option |
| No additional Mathesis staging workflow | Ordinary contributor forks send focused upstream PRs; shared issues and existing material links provide Mathesis visibility with no organization-fork service to maintain | Adopt as the operational baseline now |

No current collaboration requires the extra integration step. Reconsider the
optional workspace only when an owner can name its concrete benefit and retire
its temporary integration work when that collaboration ends.

### Evidence and owners

These are distinct observations, not one automatic completion state machine:

| Observation | Owner and evidence | Downstream meaning |
|---|---|---|
| Public request and claim of work | Contributor and shared-queue maintainers; canonical issue, scope comment and full working PR URL | Intent/coordination; no assignment bot or exclusive lock |
| Proof development and checks | Contributor; focused PR, exact head and applicable CI results | Work/check evidence; no paper verification or points |
| Admission and freeze | Upstream maintainers under current rules; accepted revision, frozen declaration, exact statement and axiom scope | Reusable formal evidence for that declaration |
| Exact paper coverage | Paper integrator; exact version/node, reviewed correspondence, pinned GID/revision and scope in existing book mapping/review | Exact, partial or pending/unknown correspondence; structural checks do not decide semantic equality |
| Independent platform verification | Existing Mathesis author-target and independent-verifier routes; eligible receipt bound to the exact confirmed target | Only this authority updates platform verified coverage and points |
| Public paper/material display | Catalog, paper publisher and Mathesis consumer; eligible public record and actual exact-version material manifest | Show available outputs and shared request links; missing video remains pending |
| Contribution attribution | Source authors/literature and upstream contributors; citation, request registration and PR history; platform receipts for platform credit | Preserve each role; GitHub attribution does not automatically become Mathesis points |

An uninspected correspondence stays unknown, not a demonstrated absence of a
proof. A paper-level Lean URL, a closed issue and a failed provider call cannot
resolve it. Existing book comparison verdicts describe the inspected comparison
at its recorded revision; `none` does not prove the entire library lacks a proof.

### Source and contribution credit

A Mathesis registration identifies **submitted through Mathesis**, the full
paper title and original authors, exact-version DOI, source theorem/section,
Mathesis version URL, current paper repository and use of the result. Retain the
original problem/literature attribution, exact statement and pinned-library
comparison. Reuse the upstream `tbook` registration; append provenance without
replacing historical markers after a repository rename. Each issue can serve
several papers and must not be relabeled as solely Mathesis-origin mathematics.
The Mathesis layer's `REQUESTS.md` gives the registration format.

Record GitHub contributors and PRs immediately. Upstream authorship and Mathesis
points have separate evidence: GitHub CI/merge alone awards no platform points.
The current independent Mathesis verifier and credit routes remain authoritative
for the platform leaderboard. Automatic reconciliation of GitHub contributions,
frozen declarations and platform credit is still required before this can be
called an automatic end-to-end credit loop.

### Exact-version and partial-scope examples

The source registration on
[request 2](https://github.com/the-omega-institute/trureturing-requests/issues/2)
identifies Haobo Ma and Wenlin Zhang's *Proof of Chen and Chen's parity-positivity
conjectures: OEIS A392664–A392666, COMMENTS, and Conjecture 2 (p. 1)*, version 2,
[DOI 10.5281/zenodo.23957560](https://doi.org/10.5281/zenodo.23957560),
Theorem 1 (`thm:main`), and its
[exact-version Mathesis record](https://mathesis-omega-research.apps.chrono-ai.fun/papers/chen-chen-step-m-eulerian-parity-positivity?version=2).
The current repository is
[mathesis-papers/10.5281-zenodo.23957559](https://github.com/mathesis-papers/10.5281-zenodo.23957559),
using the stable concept-DOI address; this does not replace the exact source's
version DOI. The historical `tbook` registration retains its original
`mathesis-papers/chen-chen-step-m-eulerian-parity-positivity` identity. Preserve
the original Chen–Chen/OEIS question and literature attribution as well as the
source paper's authors; “registered through Mathesis” adds a channel, not an
origination claim.

That registration's inspected comparison at
`2a8e85703420116c8f07e1eebebd34fa221c8d55` reports
`ChenChenParityPositivity.even_rows_positive` for integer parameters, while the
requested theorem covers real `r >= 3` and odd-row interior negativity as well.
The local book mapping accordingly keeps `paper-thm-main` at `special case`
with empty `gids`; its complete conversion stage is a content/mapping status.
Neither that stage nor the paper-level Lean source link verifies the full
version-2 theorem or earns platform points. This is existing comparison evidence,
not new independent Lean validation in this documentation task.

[Request 11](https://github.com/the-omega-institute/trureturing-requests/issues/11)
provides a second countercheck: it remains open after
[PR 30294](https://github.com/the-omega-institute/trureturing/pull/30294) merged at
`dda48614240dae7abcdcf394c668a6a876858d2f`. The issue records the admitted
`DirectedForestCompletion.directed_forest_completion` core and leaves the
broader edge-count/event-ratio/path-order scope with written-proof evidence.
Those issue reports are source-attributed evidence, not a new verification run
here. A future exact mapping must distinguish the admitted core from those
remaining clauses before changing any claim's status or closing the request.

## Phased downstream work

1. **Decision/documentation, this change.** Align the normal contribution route,
   source registration and single-story policy using the existing shared queue,
   template runtime and receipt authorities. Preserve already reviewed paper
   conversions and prepared provenance/prompt changes. No new queue, renderer,
   admission service or points system is needed.
2. **Material activation, future downstream work.** The publisher/consumer owners
   update their existing released consumer pins and enable the existing main-site
   material-discovery interface. Read back an exact-version public record and
   each advertised output/link, including the shared requests. At the last
   intake, discovery was not enabled in production and publisher pins were old;
   merged adapter/UI work is not deployment evidence. Plan only missing content:
   the inspected Chen–Chen sample has two reading languages, Beamer and an
   interactive visualization, with video still pending. Preserve its existing
   public display while the video is prepared through the current tools.
3. **One manual coverage/receipt pilot, future downstream work.** A paper
   integrator reviews one exact claim against an already published frozen
   declaration, records full revision, quantifiers, hypotheses and uncovered
   clauses in the existing mapping/review, then validates the pin and editions.
   If that claim has an eligible author-confirmed target, use the existing
   independent-verifier submission/receipt route and inspect the resulting
   platform coverage/credit. If it has no such target or no exact match, keep
   it pending without changing consent rules, awarding points or demanding a
   new upstream publication. This pilot need not redo the reviewed Chen–Chen
   content or create duplicate story/media jobs.
4. **Request adapter, future downstream work.** Establish a canonical request,
   exact target and authenticated contributor mapping using the pilot. Add
   request-scoped page/API participation over the existing verifier, then the
   durable GitHub submission/reconciliation adapter described above. No new
   upstream API, mandatory fork or model-driven GitHub operator is presumed.

The bounded next implementation delta is phase 2's existing-interface activation
and readback, followed by one manual phase-3 pilot. These are unimplemented
follow-ups, not actions performed by this documentation change. Automated
semantic binding of upstream results to exact paper claims and reconciliation
of GitHub work with independent receipts/credit remain unimplemented; the
manual pilot should establish what is missing before funding automation.

## Statistics and continued research

Count claim coverage per paper; deduplicate global mathematical results and
requests. Distinguish original authors, foundational literature, formalizers,
existing-library reuse and new proofs. A single shared theorem used by ten
papers is one formalization and ten uses. A bibliographic reference is not a
proof-dependency edge; the latter requires precise evidence.

The next research direction is a separate output: identify substantive open
questions, gaps or useful generalizations, propose concrete advances, and keep
attempt evidence. Failure to formalize the present proof is not such an advance.
Feed genuine new mathematics back as a versioned paper/result, and reuse prior
story and media when their mathematical content has not changed.
