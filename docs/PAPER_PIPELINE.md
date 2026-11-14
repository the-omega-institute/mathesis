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
slides, video and appropriate interactive illustrations. Show only built
materials of that exact public version. Academic slides are currently Beamer
PDF plus source; editable PowerPoint is not an implemented export. Short
plain-language sidecars are useful inputs, but do not certify completion of a
full explanatory story. Preserve authorship and credit in derived materials.

## Ownership

| Component | Responsibility |
|---|---|
| Mathesis | Public paper/statement identity, visibility, independent receipts, participation and platform credit |
| A paper repository | Frozen manuscript/citation, bilingual story and media sources, statement-to-theory mapping |
| Published books template | Renderer, visual/media contracts, build commands and caches |
| `mathesis-papers/catalog` | Public inventory, stable repository mapping, deterministic publication and local work plans |
| `trureturing-requests` | Shared, deduplicated formalization request discussions and handling |
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
registered requests in its materials. Readers can join the discussion and
upstream proof work there. Confirmed Mathesis targets retain their existing
proof-submission entry, independent verifier and credit rules. A contribution
made only on GitHub is attributed upstream but receives no invented platform
points. No change here bypasses an external author's existing target consent.

Keep these states distinct:

- Exact existing Lean coverage, with declaration and pinned revision.
- Written proof awaiting formalization, with a shared request.
- Partial coverage, with the precise uncovered clauses.
- A genuine unsolved conjecture/question.
- Attempt/environment failure, recorded separately from mathematical status.

Issue open/closed status is live workflow context, not verification. When a
result is published/frozen upstream, downstream must inspect its statement,
parameters and hypotheses, bind it to each exact paper claim, update the pin
and all language editions, validate and rebuild. Only receipt-backed mappings
change the verified graph/coverage or platform leaderboard. The automatic
upstream-result-to-platform binding worker is not implemented by material links.

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
