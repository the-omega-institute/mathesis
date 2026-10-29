# Author outcomes, citations and mathematical dependencies

Product design, 30 October 2026. This document specifies the next experience;
it does not claim that the proposed report sections, citation exports or
cross-paper dependency views have shipped. The implementation inventory below
distinguishes existing capabilities from the work still needed.

## 1. The author's task

The author supplies a manuscript and its bibliographic metadata. Mathesis
returns a publication decision, a mathematical analysis, useful next research
steps and specific results worth formalizing.

Keep the existing Upload → Check PDF → Details → Authors → Submit sequence.
Extract the title and abstract when possible; let authors correct them after
checking the PDF. Include authors' names, affiliations and optional ORCID,
Google Scholar and other research links. Checking the manuscript and metadata
must not become a separate exercise in constructing a theorem database.

The final submission step has one ordinary checkbox:

> Make my results public once accepted.

It starts checked. Unchecking it requests private review only, without a
publication decision or public record. Existing publication visibility and
separate approval for public formalization evidence remain in force.

## 2. One report with four parts

Use a document layout with normal headings and readable paragraphs. The four
parts are content in one report, not four jobs, dashboards or status panels.

| Report part | What the author learns |
| --- | --- |
| Decision | Accepted or Not accepted, followed by the mathematical reasons. An accepted paper links to its public record when public. Private review says Review complete and gives no publication verdict. |
| Review analysis | What the paper establishes, the scope and assumptions of its results, how it relates to earlier work, and any specific proof or exposition concerns. |
| Further directions | Concrete extensions, useful intermediate results, sharper hypotheses, applications or corrections. Explain the next mathematical step and distinguish suggestions from established results. |
| Formalization opportunities | Name the results worth formalizing, explain their mathematical value and the preparation needed, and distinguish a recommendation from work actually scheduled or completed. |

Refusals lead with the reasons. Further directions should help address those
reasons when useful; a report need not invent a research programme or recommend
formalizing a doubtful statement. Accepted reports explain contributions and
useful extensions. Neither case offers minor/major revision as a third outcome.
Do not promise automatic reacceptance after a correction.

Before a decision and final report exist, say **In review** and show the
manuscript, PDF/source and a short explanation. A partial analysis or a delivered
letter while the decision is pending must not be labelled a final acceptance
or refusal. If such feedback is available, keep it secondary and say plainly
that the decision is pending. A missing check is not a mathematical rejection.
Revision upload remains unavailable during review.

Formalization opportunities should be selective. Show the theorem's name or
manuscript label, why a reusable formal proof would help, and the next action
if one is available. Do not list every extracted supporting lemma by default.
If there is no justified recommendation, say so without inventing one. A
recommendation, an approved target, an attempted proof and a verified proof are
different facts. Only the existing verified-proof path earns public proof credit.
Private reviews can discuss worthwhile candidates without scheduling public work.

Keep one report download. Current public theorem/proof information may update
alongside the delivered report; the stored report itself remains immutable.
Author-visible recommendations must use a deliberately bounded projection of
the current manuscript and report, not unrestricted internal advice or probe files.
Historical letters are retained as delivered and are not reparsed into invented
structured findings.

The public paper page shows bibliographic information, the abstract, PDF,
accepted results, citations and available formal proofs with credit. Acceptance
does not publish the private review analysis or suggestions.

## 3. Cite a paper or a particular result

Provide a single **Cite** action with ordinary text and BibTeX. A paper citation
contains the supplied authors, title, actual publication year, Mathesis record
and public link, plus an existing DOI or arXiv identifier when available. Distinguish
the manuscript's original publication date from Mathesis's acceptance date.
Do not mint a DOI, guess a publication venue or invent a missing year. If only
the Mathesis acceptance year is known, identify it as that record's year.

A result citation adds its manuscript name/number, the source version and a
permanent link to that exact statement. The containing paper remains the
bibliographic source. Use the paper's existing name, such as Theorem 2.3
(Descent lemma), rather than inventing an eponym or exposing a database key
as the primary title. When only an internal identifier exists, use a neutral
display label and retain the full statement for disambiguation.

BibTeX should use a conservative entry type such as `@misc` when no actual
journal publication is known. A theorem citation cites the containing work
with a theorem locator in a note; it does not manufacture a separate paper.
Escaping and export must preserve mathematical notation and author names.

Credit remains explicit:

- The result retains the mathematical authors and any identified original
  problem proposer or prior source.
- A formal proof names its contributors separately. It does not replace the
  mathematical authors or imply that the formalizer discovered the result.
- Discovering a relation between two results can receive separate attribution
  under existing contribution rules; it does not transfer authorship.
- Reusing an upstream short note preserves its authors, source identity,
  original DOI, license and proof links. Mathesis is its reviewing/indexing venue.

ORCID and other supplied profiles help distinguish authors. Names alone must
not silently merge people. Citing a result does not require registration or
claiming the author's account.

## 4. Relations that mean different things

Default design: automatically identified relationships remain candidates during
private review. A public proof-dependency assertion requires examination of the
actual mathematical use and permission to disclose the underlying material.
This does not add a mandatory manual graph-confirmation step to submission.

| Relationship | Meaning and public wording | Evidence needed |
| --- | --- | --- |
| Bibliographic citation | The manuscript cites a work. Show under References. | The actual citation and an identifiable source; unresolved bibliographic matches must remain unresolved. |
| Proof use | A specified proof uses a particular result. Show under Uses these results, with the inverse Used in these proofs. | The precise source statement/version, proof location, use or substitution and applicable hypotheses. |
| Conditional assumption | The argument assumes an open conjecture or other unproved assertion. Show Assuming … explicitly. | The named assumption, its exact statement and the part of the argument that is conditional. |
| Implication, equivalence or instance | A mathematical comparison establishes a particular relationship. State that relationship in words. | The complete comparison argument, substitutions and discharged hypotheses; this is not automatically evidence of use in the manuscript's proof. |
| Related work | Potentially relevant results, without a dependency claim. | A stated reason for relevance. Similarity or shared terminology alone never becomes a proof dependency. |

For example, a paper may cite B for historical context but use C's compactness
theorem in the proof of Theorem A. B belongs in References; C is a proof-use
relation of A. If A follows jointly from C and D, retain the joint argument:
do not claim that C alone or D alone implies A. If the proof uses an unproved
conjecture E, its conditional nature remains visible even when the manuscript
is accepted.

Use **Uses these results** and **Used in these proofs** as short lists below a
statement, with theorem names, authors and source links. A small explanatory
sentence gives the role of each prerequisite or joint group. Source locations
and supporting arguments can be opened for more detail. Unknown relationships
are omitted; an empty list does not say the theorem has no dependencies.

No separate author directory, dependency editor or large graph is required
for this experience. The archive search can find papers, authors and named
results. An optional small graph can follow when enough checked relationships
exist; the lists remain the primary view.

## 5. Identity, privacy and corrections

Store exact versions even though the ordinary display remains bibliographic.
Each Mathesis result is identified by its accepted record, publication source
version, claims revision and claim identity. An imported knowledge result keeps
its upstream producer, immutable source revision and full declaration identity.
A separately proven correspondence can relate them; a similar title does not
merge them. Knowledge snapshot admission is not Mathesis manuscript acceptance.

A proof-use relation identifies the actual argument or formal proof version
whose use was checked. An alternative proof may use different prerequisites.
It is therefore inaccurate to claim a theorem logically *needs* every result
used in one proof. Keep direct evidence separate from traversed ancestry, and
never count indirect ancestors as direct uses.

For a joint comparison, retain the entire prerequisite set, parameter mapping,
hypotheses, unresolved conditions and supporting argument as one assertion.
Do not flatten a jointly sufficient set into independently sufficient edges.
Lean module imports and frozen module prerequisites do not identify theorem
proof use. Kernel verification of a target does not certify a human-readable
dependency claim unless the corresponding proof-use evidence also exists.

Publish a relation only when its source and target details are public and its
evidence is permitted for disclosure. An accepted paper becoming private must
also disappear from new inverse lists, counts, search snippets and cached graph
responses. Previously distributed citations/licenses cannot be recalled, but
they do not authorize new disclosure of private content. Private manuscript
text, query receipts and review findings never enter the public graph by default.

Revisions do not retarget old citations or transplant dependency claims to new
statements. A new accepted version needs its own checked relations. Corrections
and revoked relation judgments retain their history while disappearing from
current affirmative dependency lists. A confirmed mathematical error links to
the public correction notice; it does not silently erase the original record.

## 6. Existing capabilities and specific gaps

Inventory checked against the implementation at develop `b30dffd`:

| Capability | Current implementation | Remaining work |
| --- | --- | --- |
| Upload and metadata | Upload-first flow and editable metadata already exist. | Maintain the simple input contract; no relationship fields added to submission. |
| Decision and report | `FinalReviewReport` shows a delivered current letter and a generic formalization footer. The letter prompt already separates acceptance/refusal from internal recommendations. | Make the four report parts explicit for newly generated reports. Introduce a new pinned report contract if changing persisted prompt semantics; do not rewrite old or running rounds. |
| Formalization recommendations | Internal `Advice.formalization` includes candidates and plans. `FormalizationPlan.items` contains separate proposed/approved work with a reason. Author views deliberately omit raw advice. | Add a bounded author projection for justified candidate names/reasons after report delivery. Do not assume that all internal advice has become an approved plan, or use all proved claims as recommendations. |
| Paper citation | `web/src/lib/citation.ts` provides plain text using authors, title, acceptance record, DOI and URL. | Add BibTeX and exact statement citation support, including source version. |
| Within-paper dependencies | `Claim.depends_on` is author-confirmed and checked for known IDs, self-reference and cycles. `PublicStatements` currently prints the IDs. | Use names and links; retain the author-declared basis until actual proof-use verification exists. Do not relabel old declarations as independently checked. |
| Cross-paper assumptions | `ConfirmedDependency` records a claim's dependence on a public conjecture with the target version and claims revision. | Preserve its conditional meaning; it is not a general cross-paper theorem citation model. |
| Knowledge comparison | `KnowledgeBinding` represents joint premises and distinct equivalent/instance/implies/refutes/proof-use relations against immutable knowledge declarations. | Adapt checked evidence into a readable relationship view without treating search hits, coverage or implication as actual proof use. |
| Public relation contributions | Binding boards already require an accepted public submission, contributor access, explicit scoped sharing and independent inspection. | Reuse those rights/evidence boundaries where applicable. Existing consent to publish a paper alone does not authorize private comparison evidence. |
| Cross-paper public index | No general public index of checked manuscript-to-manuscript proof uses was established by this inspection. | Add the downstream model/projection, versioned result resolution and inverse query with visibility checks before advertising such a graph. |

Relevant contracts: [review methodology](REVIEW_METHODOLOGY.md),
[fixed knowledge snapshots](KNOWLEDGE_OPERATIONS.md),
[binding contributions](BINDING_CONTRIBUTIONS.md),
[manuscript delivery](MANUSCRIPT_DELIVERY.md) and
[upload-first submission](UPLOAD_FIRST_SUBMISSION.md).

## 7. Delivery order and acceptance examples

1. Complete the report experience: four readable parts, selected formalization
   recommendations, one download and truthful pending/private states.
2. Add academic citation tools and named links for already public statements.
   Resolve each statement against the accepted publication, including when a
   newer private draft exists. Direct citation must not require a Lean target.
3. Add checked cross-paper proof-use relations and inverse lists, adapting
   existing Mathesis evidence and existing upstream publications. No upstream
   code, release, format or publication change is part of this design.

Before calling these features complete, verify these concrete cases:

- An author can submit without naming theorem dependencies and receives the
  decision, analysis, useful directions and justified formalization advice.
- An in-review or incomplete-check case is not shown as a refusal, and a
  private review never receives a publication verdict or public citation page.
- A candidate formalization is distinguishable from an approved task and from
  a verified proof; an unsupported claim earns no proof badge or credit.
- A theorem citation still resolves to the same accepted statement after the
  author uploads a private revision or a new statement uses the same local ID.
- A bibliography citation, a similar result, a module import and an implication
  comparison do not create a proof-use edge without separate evidence.
- A joint dependence on B and C does not become two claims that each suffices.
  Dependence on an unproved conjecture remains visibly conditional.
- Making either endpoint private removes the relationship and its inverse
  from current public results, including counts and cached responses.
- Mathematical authors, original sources and formalization contributors keep
  separate credit across imported notes, local records and alternate proofs.

This is a design deliverable. No reviews are rerun, no current records are
published, and no new theorem relationships are asserted by adding this document.

## Implementation update

The author report, academic archive, citation and within-paper navigation work
is described in [Academic archive and author report](ACADEMIC_ARCHIVE.md).
That implementation preserves the distinction between manuscript-declared local
connections and the independently checked cross-paper relations specified here.
