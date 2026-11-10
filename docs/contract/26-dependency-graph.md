# §26. Dependency graph as the knowledge entry

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

`/research-graph` opens the actual result graph, never just a theory directory.
All fields opens a field overview of every loaded exact result, never an arbitrarily
selected local neighborhood. Field grouping creates no proof-dependency edges.
The initial read includes two bounded source pages (up to forty works); further
expansion rechecks the prior pages. Selecting a field opens its graph, selecting
a result opens its precise neighborhood, and All fields restores the overview.
The view composes existing Rust-owned public research, exact collected knowledge,
accepted publication and checked-proof-use projections. Versioned paper/result
identities prevent mixing source snapshots. More research re-reads the prior pages
so revoked public work is not carried forward by a client cache. Only public
publication snapshots supply ordinary claims; mismatched versions expose no claims.
Distinguish literal manuscript references, author-stated dependencies, inspected
Lean proof use, bibliography and paper membership. Only proof-term inspection adds
checked-use arrows; paper Lean links, subject placement and Mathlib paths cannot.
Source references may cycle; condense cycles for layout without claiming they form
a proof DAG. Retain original mathematical authorship and formalizer credit separately.
This graph interface queues no review or proof jobs. Collected statements now use
the separate automatic proof pipeline below; extracting Mathlib declaration
dependencies remains separate work.
