# Shared research graph: first usable slice

Mathesis indexes the references actually cited by an accepted manuscript as a
shared bibliographic foundation. Two manuscripts citing the same supported DOI
lead to one literature identity; opening that literature node finds both citing
manuscripts. An earlier author needs no Mathesis account or publication decision
to receive the attribution supplied by the source. Uploading/indexing grants no
mathematical authorship, proof credit, correctness badge or citation score.

This is a bounded foundation for a broader human knowledge library, not a complete
knowledge base. A neighborhood is a view into the same identity space across
papers. Citations may be cyclic; no topological ordering or DAG claim applies.

## Choice and reuse

| Alternative | Benefit | Missing capability / cost |
| --- | --- | --- |
| Do nothing | No implementation cost | Earlier literature has no navigable identity or visible cross-paper connection |
| References only | Cheap local attribution | Cannot open an earlier work and discover other citing manuscripts |
| Rust/Mongo shared citation identities (implemented) | Existing intake, accepted snapshots and visibility checks; forward and inverse traversal | Bounded pages and deliberately partial syntax coverage |
| Separate graph service / worldwide ingestion | Potential large-scale traversal | New authority, synchronization and access-control machinery without a current consumer |

Inspected Sisyphus at `a54987d0e3bae4c92a720f19f7a262b04e832c7f`
(Apache-2.0): `sisyphus-web/src/types/graph.ts`,
`services/graph-service.ts`, `components/NodeDetailsPanel.tsx`,
`sisyphus-api/src/schemas/models/schema-definition.ts` and
`ingest/chrono-graph-client.ts`. Reused the conceptual patterns of typed nodes,
typed edges, bounded snapshots and focused lookup. No code, 3D rendering, service
or dependency was copied. Ingest `createdBy` is operational identity and cannot
establish scholarly authorship. Sisyphus reference/topological behavior is not an
admission rule for mathematical dependencies.

PROV-O attribution/derivation and CiTO citation-purpose distinctions inform the
vocabulary ([PROV-O](https://www.w3.org/TR/prov-o/),
[CiTO](https://sparontologies.github.io/cito/current/cito.html)); this implementation
does not require RDF, SPARQL or a person registry.

## Ownership, identity and evidence

Layer 2 Rust owns identity, graph meaning, bounds, public authorization and query
assembly. Layer 3 only observes literal source syntax; binary adapters map those
observations and existing GridFS source files into the domain. MongoDB remains the
only authority. There is no cache, graph service, background crawler or derived
collection that can retain a revoked public description.

`PaperVersion.bibliography` is optional and skipped in serialization when absent.
Old versions/delivery digests remain compatible; missing bibliography means
**unindexed**, never an empty exhaustive reference list. New intake (including
producer deliveries and prepared uploads) seeds it through the existing reader
and ordinary version creation, without another author form or required chore.
Accepted publication snapshots keep observations immutable with their version.
No private referee `PriorWork` or search receipt becomes a bibliography entry.

Literature identity is chosen in this order:

1. Explicit DOI: accepted resolver/`doi:` prefixes are removed, a conservative
   literal ASCII DOI grammar is checked, and comparison is case insensitive.
2. Explicit arXiv identifier: modern `YYMM.NNNN[N]` or legacy `category/NNNNNNN`,
   optionally followed by a positive `vN`. Versioned and unversioned identifiers
   remain distinct. No DOI/arXiv alias is inferred.
3. A single literal DOI resolver or arXiv abstract URL can supply that exact
   identifier when no explicit supported identifier was selected.
4. Otherwise `reference:<record>:<accepted-version>:<literal-key>` stays local.
   Matching titles, author names and arbitrary URLs never merge works or people.

The original explicit identifiers, supplied names, title/year and source evidence
remain on the occurrence even when a DOI identifies its shared node. DOI takes
precedence over an accompanying arXiv identifier; it does not assert that the two
are aliases. A matching identifier is a source claim, not a registry lookup or
independent bibliographic authentication. Conflicting titles/authors are retained
on individual citation edges. A literature node's heading is only one visible
source description and may differ between pages; it is not canonical metadata.

Accepted manuscripts use `paper:<WP record>` identities. They are publication
records, distinct from bibliographic identities: this slice does not infer
literature-to-record aliases or DOI/arXiv equivalence. Result pages enter their
manuscript's graph and retain their existing precise result/contribution evidence.
There is no inferred mapping from a manuscript citation to a particular theorem.

## Extraction scope and bounds

The parser reads literal `cite`, `citep`, `citet`, `citealp`, `citealt`,
`citeauthor`, `citeyear`, `citeyearpar`, `autocite`, `parencite`, `textcite` and
`footcite` commands, with a star and up to two optional arguments. It reads
expanded main-file inputs, selected `bibliography`/`addbibresource` `.bib` files,
inline `bibitem` entries, or the matching main `.bbl` when no selected database
exists. A database entry without a literal citation does not create an edge;
`nocite`, including `nocite{*}`, does not create citation credit.

BibTeX braced/quoted literal fields and numeric years are supported. Ordered
`author` names split at top-level `and` and stay source strings, including
corporate names. Inline and `.bbl` prose is presented as one readable citation; author
and title boundaries are not guessed. Layer 2 derives `display_text` for public
references and citation edges, removing the structural `bibitem` label/key, braces
and supported formatting commands (`emph`, text wrappers, `newblock`, `doi`,
`url`, `href`, `bibinfo` and `bibfield`). This is a bounded presentation projection,
not a TeX renderer or new scholarly metadata. Unknown command arguments may be
omitted from display; complete raw evidence remains in an optional disclosure.
Structured titles and complete author fields stay intact. No stored source or
source digest changes when this display projection is read. Explicit `doi`, `url` and `href` arguments
inside the bounded bibliography occurrence supply identifiers/source links;
commands after the closing environment cannot identify the preceding entry.
Literal command/argument whitespace, including newlines before
`{thebibliography}`, is supported. Entries require a complete environment; a
missing/malformed closing boundary, nested environment or sectioning command
leaves an unresolved local reference and an explicit limitation, rather than
consuming later sections as evidence.
Original source downloads remain authoritative;
retained raw entries are comment-stripped observations, not byte-for-byte source
certificates.

Comments, literal/verbatim/code regions and common macro definitions are excluded
from citation detection. TeX is not evaluated. Conditional branches, custom
citation macros, `includeonly`, BibTeX string concatenation/crossrefs and BibLaTeX
`.bbl` entry formats are not resolved. Some unexecuted literal conditionals may be
included, and some actual citations may be missed. Missing or duplicate keys keep
one unresolved local occurrence with a coverage warning instead of selecting an
arbitrary identity. There is no external URL fetch or uploaded-code execution.

Bounds per version: 1 MiB of expanded source; 1 MiB total selected bibliography
text in at most 16 files; at most 100 distinct cited keys; at most 4096 UTF-8 bytes
of raw evidence per reference. Oversized entries keep an explicit raw prefix and
no inferred fields. Titles/identifiers/URLs are bounded at 1000 bytes. Author
fields are split only at complete names: each retained name is at most 300 bytes,
at most 50 names are retained, and an oversized/omitted name produces an explicit
coverage limitation while the raw evidence remains available. All source data is
untrusted. PDF bibliography
extraction is explicitly unsupported; PDF sources keep their original references.
Legacy accepted inputs are not silently rewritten or backfilled.

## Public query and interface

`GET /api/v1/research-graph` exposes the same Layer 2 service used by React:

```text
/research-graph?limit=10
/research-graph?q=Earlier&before=WP-2026-0010
/research-graph?focus=paper%3AWP-2026-0001
/research-graph?focus=doi%3A10.1234%2Fold
/research-graph?focus=arxiv%3Amath%2F0301234v2
```

Parameters are bounded at 500 bytes. Focus and text search are separate query modes.
A source-local reference opens its one accepted source directly, including older
records; its identity does not imply a cross-paper match. Global, text-search and
DOI/arXiv focus requests return 1–20 **matching public accepted source records**
(default 10), each with at most 100 observations. Typed supported DOI/arXiv values
use the same identity predicate as focus; other text searches supplied titles,
individual complete author fields, identifiers and retained citation prose.

The existing record port has a `search_research` operation. Mongo selects all records
after the requested cursor, ordered by record identity, and joins live public visibility.
It applies no text or identifier candidate filter. Rust's shared `ResearchSearch`
predicate confirms accepted input (including the legacy fallback), purpose,
exact identity and matching fields **before a record consumes a page position**.
Thus every domain match survives adapter selection for every supported query and
source string: the only earlier restrictions are the requested cursor and the live
visibility required by that same predicate. This includes ASCII queries over Unicode
source text, where Rust lowercase expansion can differ from Mongo caseless regex,
as well as Unicode queries and exact identities. Superseded versions and DOI substring
candidates cannot count as matches. Memory storage uses the same predicate.
Neither path limits discovery to an
unrelated source prefix. No derived collection, migration or new service is needed.

Output is bounded by page size; Mongo cursor batches contain at most 20 candidates.
A 10-second database limit and an outer 10-second elapsed deadline bound query work,
including cursor reads. On expiry the request fails explicitly; it never reports
an empty result or a successful partial scan. Literal bibliography substring
search scans visible candidates without a dedicated full-text bibliography index, so large collections may
reach this deadline. Existing record/submission identity indexes support ordering
and visibility joins. Layer 2 re-reads live visibility and immutable accepted inputs
before producing any metadata or edge.
No newer private source, review, digest, operational identity or hidden supporting
record can choose the literature node's heading. Every returned edge has both
endpoints in that response. Direct focus on a private/absent manuscript returns
404; inverse focus with no visible supporting sources is empty.

`next_before` continues matching source results, preserving focus or search; unrelated
publications do not create empty result pages or reset known-work access. `coverage`
reports each returned public version and its indexing limitations. An empty result
means no matching currently indexed accepted input within the completed query;
it does not establish worldwide or full manuscript reference coverage. Concurrent
visibility changes can remove an item between selection and projection. Do not
merge old client pages into a durable public cache: responses use
`Cache-Control: no-store`, and the UI replaces each page on navigation so fresh
visibility is checked.

The English Research graph entry opens directly into a visual map, with no separate
search form. Earlier literature appears on the left, citing Mathesis papers on the
right, and source-supplied authors appear beside each work. Visual arrows run from
earlier work to the papers citing it, as the caption explains; API citation edges
retain their manuscript-to-reference direction. Selecting a work highlights its
connections and opens full titles, author credit, source links and clickable
neighbors alongside the map. Source evidence and coverage explanations start closed.

The map shows at most eight references at a time, with explicit Previous/Next
reference controls, a range/count and a scrollable canvas. Selecting a reference
from the details panel opens its map page. This local layout pagination is distinct
from the API's next group of source papers; no earlier response is retained as a
client-side graph cache. Repeated citations between two nodes share one visual
line, while their separate source occurrences remain available in the evidence.
Existing text-query URLs remain supported. Empty public responses show an honest
empty state rather than example works.

Paper pages show retained raw reference evidence; result pages link to the manuscript
neighborhood alongside their existing local statement dependencies, source-history
roles and verifier-backed proof credit. This map does not add theorem or formalization
nodes to the shared API. There is no force layout, fractional credit or added upload step.

## Growing rigorous mathematical dependencies

Keep four relations distinct: source-declared citation; author-confirmed local
statement dependency (open assumptions remain conditional); source-declared
contribution history with its existing problem/mathematical-proof/exposition/
formalization/correction roles; and independent verifier receipts tied to exact
confirmed targets. Indexing creates only the first. A verified theorem in a
citing paper does not verify its citations as proof use.

A general cross-paper implication claim needs exact versioned result identities,
explicit source/target correspondence, complete jointly used premises, discharged
hypotheses, evidence scope and an admission authority. A source-declared claim
must remain distinguishable from an independently inspected argument and a
verifier-checked dependency. Only dependency relations that require acyclicity
should reject cycles, within their precise scope; citation cycles stay valid.
Imports, similarity, authorship, creation timestamps and module order cannot
supply such an edge. Reuse the current confirmed-target/independent-verifier and
source contribution contracts instead of creating a second proof-credit ledger.

## Validation and demonstration

Synthetic tests connect two accepted public manuscripts through one historical
DOI, preserve conflicting descriptions, withdraw visibility, introduce a newer
private version, reject fuzzy identity merging and page matching results correctly even behind
551 unrelated publications. Exact focus and title/author/typed-identifier searches
all find the two citing sources without empty-page traversal.
Mongo integration tests exercise ordinary source intake, persistence/reconnect,
visibility before pagination and immutable accepted-version reads. Parser fixtures
cover BibTeX, inline/.bbl prose, archive inputs, preamble `addbibresource`, unused
entries, ambiguous/missing keys and limits. React tests exercise earlier-work
entry, two incoming citations, names/source links, query routing, coverage and
unsafe-link handling. Map interaction tests cover selection, reference pagination,
fresh responses removing previously selected work, duplicate visual edges retaining
their evidence and isolated papers without invented connections.
A shared inline/bbl fixture is checked against actual Rust
parser/domain output and consumed by DOM tests for paper references, relationships
and focused work with evidence both closed and open. Existing result/contribution/verifier behavior remains under
its existing tests. These are local fixtures, not live test publications.

## Checked contribution use alongside the citation map

The result-specific `uses` projection is separate from the citation map. It links
an independently checked final proof to exact named contributions retained from
another public pinned result. The verifier's elaborated proof-value inspection
supplies the `checked_proof_term` basis. Both directions retain original author,
contributor, license and whole selected-context evidence. The source goal may
remain unfinished, and an individual helper is not presented as sufficient.
Citations and imports alone never enter this projection. Fresh reads and bundles
recheck both endpoint visibility and exact source/target pins. No graph-editing
step is added to author submission.
