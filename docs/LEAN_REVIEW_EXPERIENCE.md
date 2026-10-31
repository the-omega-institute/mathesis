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

Supporting source comments can include issue references. Sources containing quoted
literals retain the checker's conservative hash restriction; target and solution
adapters keep their strict scan. Preflight acceptance alone is never verification.

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

## Finite formalization and saved propositions

CMA performs manuscript checking, advice, target preparation and proof attempts.
The venue decides publication from saved evidence. Only the independent Lean
verifier decides whether a submitted proof checks.

After confirming an approved public paper target, the submitting author can
allow 1–20 total generations with an expiry within 30 days. The result page and
author workspace show consumed work, remaining authorization and pause/resume.
Resuming preserves the original expiry and consumed allowance. An extension is
an explicit author action. Source changes, expiry and pause fence new provider
work; retries recover the same saved provider identity and input context.
Provider input retains the original contributor snapshot, so a later profile
rename cannot change the bytes of an admitted helper context. Public credit
links resolve the contributor's current profile.
Empty, malformed and rejected outputs consume a round and eventually stop.
Public conjectures retain their existing single venue attempt per target.

A participant or later CMA round can save a named auxiliary proposition through
the ordinary attempt and independent verifier. The proposition is checked against
its own exact type in self-contained supporting modules under the pinned workspace.
The type may use definitions from those exact modules. They compile before the
auxiliary target and cannot import Target; ordinary author targets retain their
existing strict profile.
It remains available when the main goal is unfinished. Unchecked names, progress
text and rejected final solutions never become checked contributions.

A later proof can select up to eight checked propositions. Their exact module
bytes, original source/result pins, license and contributor attribution travel
with the proof and its downloadable verification bundle. Reuse creates no extra
leaderboard points. A source deposit retains its separate research-author,
formalizer and submitter identities. No historical helper evidence is invented.

Result pages show both directions of checked contribution use. The verifier
inspects elaborated proof values and reports only requested proposition constants
actually reached through the submitted proof context. Unused imports, bibliography
citations and manuscript-declared dependencies retain their distinct meanings.
The view identifies the exact source version and the complete selected joint
context; it does not claim individual sufficiency or that an unfinished source
result was proved. Every fresh public list, profile, count and bundle checks the
visibility and current immutable pins of both ends. API responses use no-store.

The REST, OpenAPI, web, CLI and MCP controls share the same Layer 2 authorization
and verifier path. See [API.md](API.md) and the contributor SDK documentation.
The verifier suite includes real pinned-Lean checks for helper scope, failed final
proofs, actual versus unused imports and attribution rejection; these run in the
existing Linux verifier image gate, never through a model or fixture verifier.

A control request that changes an existing total or expiry must explicitly set
`change_allowance: true`. Pause/resume sends `change_allowance: false` and the
saved limits; a stale request with different limits returns a conflict. The
first grant defines its limits. Consumed generations are always preserved.
When the confirmed target changes within the same manuscript version, the author
must explicitly save the allowance for the replacement target. This retains
consumed generations and saved limits unless explicitly changed, assigns a fresh
round identity and fences the old target; merely resuming cannot authorize the
replacement.
