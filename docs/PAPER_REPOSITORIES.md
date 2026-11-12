# Public paper repositories and catalog

Mathesis remains the authority for collected papers, statement identity, version
selection and verification state. GitHub repositories, the organization profile
and the catalog are projections. Zenodo stores archived versions. A projection
can be rebuilt from a successful public snapshot.

The organization name is a deployment option; the current deployment is the
GitHub organization `mathesis-papers` (display name Mathesis). Its repositories are:

- `.github`, containing the organization profile README;
- `catalog`, containing `works.json`, `nodes.json`, `stats.json` and snapshot metadata;
- `paper-template`, containing the static tree used by each paper repository;
- the shared Lean library, currently `the-omega-institute/trureturing` at the
  revision named by each public source link.

The generator renders one long-lived repository tree for each in-scope collected
work. Rendering a tree does not publish it: which rendered trees become
repositories is the publisher's decision. The
stable work id is the repository name when it satisfies GitHub's repository-name
rules. Otherwise the generator uses `paper-` followed by the first twelve
hexadecimal characters of the SHA-256 of the work id. An explicit mapping file
can override this choice and is retained as the durable mapping once repositories
exist. A title change never changes the repository name.

A current collected statement node is exactly
`collected:<work>:<version>:<claim>`. The same string is used by the existing
Mathesis collected-proofs code. An accepted-record or other non-collected row
uses `record:<record>:<version>:<claim>` because the public server exposes no
separate accepted-node identity. `lineage` keeps the prefix and omits the
version. The selected current public version is counted;
previous versions are recorded in `works.json` and `CHANGELOG.md` and are not
counted. Attempts, repeated builds and several receipts for one statement do not
create more nodes. Different works remain separate even when their text or title
is similar.

Each node records the work, version, claim, label, kind, complete statement,
macros, status, target digest, toolchain, receipt, proof basis, original authors,
formalization contributor and attribution. The paper-level Lean source link and
its exact revision are separate from verification. A verified node is bound to
existing library declarations or to new Lean. An unverified node has no proof
basis. Directory rows for accepted records, including conjectures, remain in
`nodes.json` and are excluded from the collected-work proofs cross-check. Every
node carries `scope`: `in_scope`, `out_of_scope`, or `requires_author_consent`;
nodes outside publication scope remain recorded but are excluded from profile
counts.

The catalog statistics have fixed scopes:

| Metric | Scope |
|---|---|
| Public papers | In-scope current public versions of collected CC-BY-4.0 works |
| Registered statements | In-scope selected-version nodes, deduplicated by node identity |
| Independently verified statements | In-scope nodes with an exact target and public proof receipt match |
| Verified from existing library declarations | In-scope verified subset whose receipt basis is `library_declarations` |
| Verified with new Lean | In-scope verified subset whose receipt basis is `new_lean` |
| Verified counterexamples / disproofs | In-scope public disproof receipts, shown separately |
| Open formalization targets | In-scope confirmed targets without a verified proof |
| Statements awaiting a formal target | In-scope nodes without a confirmed target, including a needs-attention sub-count |
| Papers with Lean source | In-scope papers with a public paper-level source URL pinned to an exact commit |
| Papers with a DOI | In-scope papers with a checked exact-version DOI in the public projection |

The profile shows counts side by side, the RFC 3339 UTC snapshot time, a table by the
first two digits of the first MSC code, recent verified nodes with exact Lean
links, recent papers, and links to the full catalog, Mathesis and the
formalization directory. It never prints a completion percentage.

A paper repository has this tree:

```text
<repository-name>/
├── README.md
├── paper.json
├── CITATION.cff
├── CONTRIBUTING.md
├── manuscript/
├── paper.pdf
├── formalization/nodes.json
├── verification/
├── LICENSES/
├── CHANGELOG.md
└── .mathesis/generated.json
```

The README preserves the recorded title and authors, abstract, frozen PDF link,
checked DOI and concept DOI when present, license, citation, version and statement progress.
Authors and formalization contributors are separate. `formalization/` maps
statements to evidence; it does not copy the shared library. A paper-specific
Lean environment appears only when the library lacks required Lean, and its
pins come from the verifier environment. The generator does not emit
`lean-toolchain`, `lakefile.toml`, `lake-manifest.json` or a `Paper/` directory
for the current phase. `LICENSES/CC-BY-4.0.txt` is the official Creative
Commons legal code. `paper.json` has `files.pdf` and `files.source` objects with `path`, `bytes`,
`sha256`, `public_sha256`, and nullable `matches_public`; it does not embed the
statement nodes. Generated manifests list generated paths and SHA-256 values.

Phase 1 includes collected works whose public record says `CC-BY-4.0`. Accepted
author-submitted records are included in the snapshot and catalog with
`publication: "requires_author_consent"`, but receive no repository tree. A
public repository or archive record cannot be withdrawn by the generator, while
CLAUDE.md §4 permits a later privacy change. Pull requests and CI do not confer
verification or credit. Contributors submit attempts through
[`docs/CONTRIBUTE.md`](CONTRIBUTE.md) and `sdk/contribute`; repository changes
are for explanations, errata pointers and reproducibility material. Copying an
upstream result creates no new contribution. Accepted directory rows are retained
with their explicit scope and are not counted as public papers or registered
in-scope statements.

The generator displays an existing checked exact-version DOI. It never registers
one. One party registers DOIs for a given paper version. Zenodo API deposits are
made by the publisher described below; the Zenodo–GitHub release integration
remains off because it mints a DOI per release. Carry-over of a receipt to a new version is allowed only in a later
publisher when the target digest is identical and the public Mathesis projection
states that relationship.

`sdk/catalog` first writes a complete snapshot to a temporary sibling directory
and replaces the destination only after traversal, proof cross-check and all
writes succeed. Any non-retryable HTTP error, exhausted retry, malformed page,
archive violation or hash mismatch exits non-zero and leaves the prior snapshot.
The fetch phase performs anonymous GET requests and keeps frozen bytes
byte-identical. Rendering is offline and deterministic. `check` recomputes the
statistics from the rendered catalog.

The output schemas are stable versioned JSON objects:

- `snapshot.json` has `schema_version`, `generator_version`, `api_base`,
  `captured_at`, listing page/row counts and row totals.
- `works.json` has `works`, with repository mapping, record, DOI, `concept_doi`, current and historical
  version information, publication scope, authors, license and source
  descriptors.
- `nodes.json` has `nodes`, with the versioned identity, lineage, statement,
  status, source and receipt fields.
- `stats.json` has the fixed metrics above and `by_msc` fields for papers,
  registered, verified and open targets.
- `.mathesis/generated.json` has generated paths, SHA-256 values, generator
  version and a snapshot identity.

The public profile links the organization `catalog` repository. Site links use
the public paper route `/papers/<record>?version=<version>`, the formalization
directory `/formalizations`, and the contribution route `/contribute`.

Phase 1 is the local generator, its schemas, documentation and tests. The
publisher is later work: this client program, run by a workflow of the
organization's `catalog` repository, with the GitHub and Zenodo credentials held
as organization secrets. It pushes the core repositories, creates one repository
for each accepted author-submitted paper whose author gave archive consent,
deposits the frozen files through the Zenodo API and publishes an archive index.
The Mathesis server holds neither credential. It records the author's consent
and licence, and attaches a DOI only after it has verified the public archive
record against its stored bytes. Those operations are outside phase 1.
