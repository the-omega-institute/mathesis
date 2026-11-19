# §26. Dependency graph as the knowledge entry

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

`/formalizations` opens the subject directory. Each subject has its own address,
`/formalizations?msc=<two-digit MSC code>`, and its own dependency graph. The directory
loads paper classifications without fetching or drawing a combined library graph.
A subject read selects only its papers and exact-version proof targets, then adds
those results' actual prerequisite closure. Cross-subject foundations stay visible
where needed; unrelated results and later uses from other fields are excluded.
Changing subjects resets the graph and updates the address. Following a prerequisite
keeps the current subject, rather than silently switching to another field.
Old module-focus links resolve into a matching subject address. `/research-graph`
continues to redirect to formalizations; precise statement/citation neighborhoods
remain available separately. There is no all-fields combined DAG.

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

Each subject graph consumes the current published shared Lean evidence
using its manifest and compact startup view. Fetch
the existing public artifacts with CORS and revalidate the mutable manifest/index;
check content digests, exact release/source identities, complete node order and
Lean admission records before rendering. Preserve published module dependencies
separately from affinities and Blueprint relationships. Solid nodes represent the
shared library's admitted formalizations; hollow nodes retain its explicit Open
state. Mathesis supplies an independent graph for each MSC subject in
the paper directory, laid out independently of upstream coordinates. Exact paper
MSC metadata overrides editorial topic placement. Project directories and internal
upstream topic labels are not shown as reader-facing disciplines.
A subject collection starts with that subject's Mathesis public paper source modules and
their actual prerequisite closure. It does not reproduce every upstream project
node or its Atlas presentation. A directly linked module can open its own closure.

Connect papers only to their recorded Lean module, without interpreting that link
as whole-paper certification. Add exact-version platform proof targets and matching
verification receipts to this view; dashed paper-context links are distinct from
proof dependencies. GitHub request links remain contribution links: neither a link
nor an issue closure is converted into a proof state. Importing these data starts no
review/build and grants no Mathesis receipt or leaderboard credit. Existing exact
statement and citation views remain available through their links.
