# §12. Solving and the leaderboard

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

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
  commands, and imports other than Mathlib, Target, submitted supporting
  modules validated under §14, and prepared libraries declared by the verifier
  workspace (never imported by a target; see `docs/LIBRARY_BINDING.md`).
- The verifier image compiles its prepared library from source at the one
  revision pinned in `infra/verifier/lakefile.toml`. That revision,
  `library-modules.txt` and `library-proof.json` change together, and the image
  build must check the proof of `library-proof.json` in the prepared workspace.
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
- Continuous public paper formalization requires a submitter-authorized lifetime
  allowance of 1–20 generations and expiry within 30 days. Authorization, consumed
  rounds and frozen helper context live in the revision-fenced SolveFile. Pause,
  expiry and source/target changes fence provider work; resume retains consumption,
  expiry, admitted input bytes and original managed handles. Empty, malformed and
  rejected responses spend a round. Public conjectures keep their single venue attempt.
  A corrected target requires explicit author rebinding; it retains lifetime
  consumption and saved history and uses a fresh round identity. Pause/resume
  cannot authorize a replacement target or change saved limits.
- Named intermediate propositions use the same Attempt, source blobs and independent
  receipt. Their exact standalone type and named theorem ownership are checked in
  submitted self-contained modules. Auxiliary types may use definitions from those
  exact modules; auxiliary modules compile before their generated target and cannot
  import Target. Ordinary author-target restrictions stay unchanged. They survive an unfinished main target without
  becoming a final proof/disproof or earning new leaderboard points. Reuse copies
  exact source bytes under the same toolchain and preserves original pins, license,
  research authors, formalizers and entrant attribution.
- Cross-paper checked-use links derive only from verifier inspection of elaborated
  proof values for explicitly selected propositions. The forward/inverse projection
  identifies exact versions and complete selected joint context, never individual
  sufficiency. Citations, manuscript declarations, source histories and argument
  assessments keep their separate bases. Fresh public counts, profiles and exports
  hide attempts whose reused source is private or stale; no legacy evidence is inferred.
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
- Public-facing review copy says “AI reviewed”; provider details belong in staff execution records.
