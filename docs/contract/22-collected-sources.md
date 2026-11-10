# §22. Collected sources and formalization participation

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

Collected works join the public citation graph by their stable work id and exact
version. A source-SHA-bound bibliography sidecar is read deterministically from
the existing source archive; admin backfill neither mutates delivery receipts nor
queues review, compilation or proof work. Graph pagination traverses accepted
records and then collected works, filtering matches before pagination. Citation
edges remain source-declared references, never proof dependencies. Problems
named by current public collected works enter the problem library at read time
under their own operator/producer/source/name identity; see
`docs/PROBLEMS_AND_NOTES.md`.

The Lean graph includes published collected Lean sources alongside independently
checked propositions and confirmed targets. A source link alone is not a checked
target or a new contribution. Only confirmed open targets offer target-specific
proof submission. Paper formalizations have a separate leaderboard: first verified
proofs count, with authorship, visibility and target validity enforced by the same
rules as existing contribution profiles. Importing a paper or Lean link awards no
new Mathesis proof credit.
