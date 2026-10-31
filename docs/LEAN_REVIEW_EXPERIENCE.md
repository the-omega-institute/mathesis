# Manuscript review, formalization and participation

Authors submit a manuscript, check its PDF and confirm bibliographic metadata.
The ordinary review page presents a final outcome, mathematical analysis,
suggestions and formalization opportunities. Review-only submissions remain
private and receive feedback without a publication decision.

## What authors and readers see

| Point in the process | Author view | Public view |
| --- | --- | --- |
| Draft | PDF, editable title/abstract, authors and links | None |
| Review underway | Current status; saved work and any temporary service delay | None |
| Final report | Applied publication decision or private-review completion; analysis, improvements and further directions | Accepted public manuscript only; private reports stay private |
| Formalization recommended | The particular result and the proposed mathematical work | No verified-proof claim |
| Author approves a public result | Preparation of an exact Lean statement, independently compared with the manuscript, for the author's confirmation | A participation target becomes available after confirmation |
| Proof work underway | Saved target, attempt progress and available next-step information | Current confirmed target; unsuccessful attempts remain private |
| Independent verification passes | Verified proof and its scope | Exact confirmed target, proof source and contributor attribution |

Manuscript result names retain their supplied numbers or titles. If extraction did
not recover a number, a section title does not replace the result's type and a
number is not invented. Source extraction can miss proofs written in prose. A
current source audit can establish their eligibility for formalization, while
leaving the original extraction and confirmed statement data intact.

## Existing Lean work

An uploaded Lean attachment is evidence for review. An attributed source deposit
is saved source. Neither receives a Mathesis verification badge automatically.
Existing upstream formalizations may be displayed with their original contributor,
version, source link and stated scope. They do not acquire new authorship merely
because someone uploads or indexes them.

To obtain Mathesis verification, a proof must match a current confirmed public
target and pass the independent verifier. This is distinct from the private
post-review Lean probe, which does not publish a proof or award contribution credit.
The theorem proved in Lean must match the mathematical statement: a proof of an
older special case cannot verify a stronger new result.

## Participation and credit

- Original mathematical authors retain their authorship and source references.
- Source-supplied contribution history can separately attribute the problem,
  mathematical proof, exposition, formalization and correction, with evidence.
- Signed-in contributors can submit a proof or disproof of a confirmed public
  target. Verified contributions identify the entrant and any separately supplied
  research/formalization attribution. Uploading alone earns no proof credit.
- The first independently verified successful attempt receives the existing
  profile credit; later verified proofs are still displayed as also verified.
  Paper formalizations and conjecture solutions have distinct credit categories.
- Confirmed error reports identify the reporter and follow the existing correction
  and re-review process. Unconfirmed reports are private.
- Supporters fund venue work; funding does not make a supporter the proof author.

## CMA continuation and current limits

CMA performs manuscript checking, advice, target preparation and proof attempts.
The venue decides publication from saved evidence. The independent Lean verifier,
not CMA's assertion of success, decides proof verification.

An approved paper formalization can continue from its previous submitted Lean
source and verifier feedback. A service interruption retains the same task identity;
the author should not upload another copy to recover it. Each execution has a saved
deadline. The present continuous formalization loop does not yet provide an author
budget/pause control or a complete sequence of individually verified intermediate
lemmas. An empty attempt can lead to another attempt without a reusable intermediate
artifact. Do not describe this as a finished service for building a fully verified
library one lemma at a time.

The research graph currently represents citations, manuscript-declared result
dependencies and source-attributed contributions at their respective scopes.
General cross-paper, verifier-backed theorem dependency edges and automatic
per-lemma credit are not inferred from citations, imports or model prose.
