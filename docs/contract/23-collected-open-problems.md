# §23. Collected open problems

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

Trusted collection operators (`WISHPOOL_INTERNAL_SUBJECTS`, the pre-reviewed
collection rule) submit curated open problems through `/collected-problems` as
`mathesis.collected-problem.v1` envelopes. Versions are immutable Mongo documents
fenced by their named predecessor; exact retries return the same receipt and a
changed payload under a published version is refused. The entry's identity is the
collected-problem key (operator, producer, source kind, locator, name), kept across
versions, so a curated problem and the collected works that name it are one
problem-library entry. Never merge by similar titles or statements. Intake admits
only `open` or `partially_resolved` with one to ten dated evidence items recording
what the curator opened and saw. Public projections show that status as the
curator's dated reading (`curated_open`, `curated_partially_resolved`), never as a
Mathesis audit result; a work naming the problem does not settle it. Original
posers and sources are reported names. Operator text is data. Collection runs no
referee, Oracle, audit or model, creates no record, decision, proof job or
credit, and imports nothing by itself. See `docs/COLLECTED_PROBLEMS.md`.

A version may carry the curator's own Lean target: `import Mathlib`, the
curator's definitions and one `def wishpool_target_prop : Prop`, with a required
plain-words correspondence. It is delivered, never generated. One durable
verification-lane job per version has the independent verifier check an
elaboration witness, `p → p` for the curator's proposition `p`, and attaches the
outcome once: `elaborated`, or `refused` with the verifier's reason. An
unreachable or late verifier writes nothing. Public projections show the Lean
text only when elaborated, always as the collection operator's formalization that
the original poser has not confirmed, and state that elaboration shows a
well-formed proposition, never a faithful one. `vocabulary` is a textual reading
of whether the target declares its own definitions. A curated target is not a
confirmed public target: it opens no attempt, verification of a solution, credit
or leaderboard entry, and a correction is a new version.
