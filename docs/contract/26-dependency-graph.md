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

The default formalization graph consumes the current published shared Lean evidence
using its manifest and compact startup view. Fetch
the existing public artifacts with CORS and revalidate the mutable manifest/index;
check content digests, exact release/source identities, complete node order and
Lean admission records before rendering. Preserve published module dependencies
separately from affinities and Blueprint relationships. Solid nodes represent the
shared library's admitted formalizations; hollow nodes retain its explicit Open
state. Mathesis supplies its own presentation, grouped by the same MSC subjects as
the paper directory and laid out independently of upstream coordinates. Exact paper
MSC metadata overrides editorial topic placement. Project directories and internal
upstream topic labels are not shown as reader-facing disciplines.
The default collection starts with Mathesis's public paper source modules and
their actual prerequisite closure. It does not reproduce every upstream project
node or its Atlas presentation. A directly linked module can open its own closure.

Connect papers only to their recorded Lean module, without interpreting that link
as whole-paper certification. Add exact-version platform proof targets and matching
verification receipts to this view; dashed paper-context links are distinct from
proof dependencies. GitHub request links remain contribution links: neither a link
nor an issue closure is converted into a proof state. Importing these data starts no
review/build and grants no Mathesis receipt or leaderboard credit. Existing exact
statement and citation views remain available through their links.
