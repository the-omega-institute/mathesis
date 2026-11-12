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

The local generator and the external publisher are separate programs. The
publisher in `mathesis-papers/catalog` owns GitHub writes with its GitHub App.
Archive deposit automation, accepted-record repository creation after consent,
and Zenodo credentials are outside this package. The Mathesis server holds no
publisher credential and attaches a DOI only after public archive verification.

## Automatic publisher and organization site

The offline renderer also emits `site/index.html`, `site/doi/<DOI>/index.html`,
`site/CNAME` (`papers.trureturing.org`) and `site/.nojekyll`. The index shows
all current in-scope papers, the snapshot time, scoped counts and Mathesis links.
Repository/site links appear only for entries in the durable `repositories.json`
mapping, not merely because the renderer proposed a repository name. DOI pages
redirect to the mapped paper site, otherwise to Mathesis; concept DOIs do not
create exact-version redirects. Unsafe DOI paths and conflicting mappings fail
rendering atomically. No current time or network enters rendering.

`catalog/archive-index.json` has exactly the `mathesis.archive-index.v1` shape
specified by `docs/PAPER_ARCHIVE.md` in PR #120: `schema`, `generated_at` (snapshot
RFC 3339 time), and sorted `entries` of `kind`, `id`, positive integer `version`,
`doi`. `kind` is `collected` or `record`; entries contain only exact-version DOIs
already supplied by the public projection. Concept DOIs are excluded. This
publisher does not deposit archives or register DOIs. Index discovery still
verifies stored bytes and consent before attaching a DOI on the server.

Every work gets `catalog/materials/<work>/materials.json` with schema
`mathesis.catalog-materials.v1`, `work`, `version`, nullable `repository_url`,
`site_url`, `manifest_url`, nullable exact-version `doi`, `reading`, `slides`,
and `visualizations`. Unmapped works have null repository/site/manifest URLs and
empty material lists. A mapped repository without a manifest has empty lists.
A consumed manifest also sets `manifest_available: true`. `--materials DIR`
reads `DIR/<repository>.json` offline; identity and all material URL boundaries
are checked. The Mathesis public record remains the DOI authority.

Paper repositories publish `mathesis-materials.json` at the site root. Its
schema `mathesis.materials.v1` contains `work`, integer `version`,
`repository_url`, `site_url`, nullable `doi`, `reading` (language → absolute HTML
URL), `slides` (objects with `id`, `language`, nullable `pdf_url` / `notes_url`),
and `visualizations` (objects with `id`, `url`). Only built output is advertised.
The Mathesis layer emits it through the upstream optional `site.post_build`
interface and redirects a single-book paper's root to the original nested book
landing. All original book URLs continue to work. This package does not change
the Mathesis server to read these materials.

The catalog repository owns a daily/manual publisher authenticated by a
short-lived GitHub App token. It builds this SDK at an exact Mathesis commit,
stages snapshot → fetch → render → check, downloads repository manifests
anonymously, renders/checks again with a publication plan, and fills/checks all
new paper skeletons from an exact template commit. No GitHub write occurs until
all staging checks pass. A full output hash manifest is checked by `check`.

`publish.json` is an explicit list of work ids; initially it contains only
`chen-chen-step-m-eulerian-parity-positivity`. Publication eligibility remains
collected CC-BY-4.0. The allow-list grants repository creation, not mathematical
content generation. Existing mappings remain durable. New repositories come
from `paper-template`; Pages uses workflow mode and one issue carries the
layer's reading/media checklist. Template provenance and imported paper identity
make retries safe across partial setup. Existing populated books are preserved;
new paper versions require reviewed projection/book updates, not another fill.

Catalog files are committed to `catalog`, profile README to `.github`, and the
site to `mathesis-papers.github.io`, retaining `CNAME` and unrelated repository
files. Commits occur only for changed bytes. Ref updates are atomic per
repository and never forced; GitHub offers no transaction across repositories.
An apply-stage API failure may leave completed steps published and is resumed
by the next run; a staging failure publishes nothing. Organization-wide
concurrency prevents overlapping publisher runs. The App must be installed for
all organization repositories (including future ones) with administration,
contents, pages, workflows and **issues write**. Issues write is additionally
required for the preparation issue. Neither GitHub nor Zenodo credentials live
on the Mathesis server. App installation and all remote writes belong to the
dispatcher for this work package.
