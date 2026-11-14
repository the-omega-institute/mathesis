# Mathesis public catalog generator

`mathesis-catalog` reads only anonymous public Mathesis GET routes. It never
writes GitHub or Zenodo. A snapshot is a complete, immutable input for the
offline renderer.

```sh
# Collect every page and every formalization-directory status.
mathesis-catalog snapshot \
  --api-base https://mathesis-omega-research.apps.chrono-ai.fun \
  --output /tmp/mathesis-snapshot

# Download frozen public files, checking every hash supplied by the projection.
mathesis-catalog fetch --snapshot /tmp/mathesis-snapshot

# Render the catalog, profile, paper trees, formalization tree and static template.
mathesis-catalog render --snapshot /tmp/mathesis-snapshot \
  --output /tmp/mathesis-output --organization example-mathesis \
  --site-base https://mathesis.example

# Recompute stats and fail if a generated catalog was changed.
mathesis-catalog check --output /tmp/mathesis-output
```

Use `--mapping repositories.json` on `render` for a durable JSON object mapping
work ids to repository names. `--organization` is the GitHub login used in
links; `--display-name` (default `Mathesis`) is the profile title. `--organization` and `--site-base` are explicit
render inputs; the renderer reads no current time. `--api-base` may be either
the public site root or its `/api/v1` root.

`fetch` is deliberately separate from `render`. Rendering is deterministic and
offline: it reads the snapshot and any files downloaded below
`snapshot/files/`. A public file descriptor without a SHA-256 is retained and
marked `hash_verified: false`; a supplied hash is mandatory evidence and a
mismatch aborts the work. Source archives are unpacked with bounded size/count
and path checks, and links are never followed.

The output has `catalog/works.json`, `nodes.json`, `stats.json`,
`snapshot.json` and `README.md`, the organization `profile/README.md`, one
`papers/<repository-name>/` tree per CC-BY-4.0 collected work or consented
accepted author paper, and the static `template/` tree. Author papers require
the exact public archive consent marker and frozen-file descriptors from their
current public details. A listing flag or licence alone is insufficient.
Unconsented and legacy records stay `requires_author_consent`, without copied
files or material links. A concurrent change aborts snapshot replacement.
Original source bytes are retained in `source.archive`, with the public hash
and size checked; extracted archives, single TeX files and PDF-only uploads
remain readable in `manuscript/`. Paper manifests identify their own content,
so refreshing the catalog time does not rebuild unchanged papers.

`src/msc_labels.json` is a copied, package-local projection of the English
MSC labels used by `web/src/data/theory-areas.json` at package creation time.
The SDK does not read `web/` at build time. Source builds now use the full
Mathesis checkout to cross-check the generated verifier pins against `infra/verifier/`.

The renderer additionally produces `site/` for `papers.trureturing.org`,
`catalog/archive-index.json` in the exact `mathesis.archive-index.v1` schema,
`catalog/repositories.json`, `catalog/materials/<work>/materials.json`, and
`site/materials-index.json` (`mathesis.materials-index.v1`).
The org site has scoped counts, the snapshot time, paper links and exact-version
DOI redirects; its CNAME is always preserved in publisher synchronization.
Repository links require a durable mapping entry. Prospective repository names
in `works.json` continue to identify renderable paper trees.

Use `--materials /tmp/repository-manifests` to consume locally downloaded
`<repository>.json` manifests (`mathesis.materials.v1`). The renderer performs no
fetch itself. Missing manifests produce empty material lists; malformed or
wrong-version manifests fail rendering without replacing the prior output.
Manifest identity, site URL boundaries and exact versions are checked; DOI comes
from Mathesis. See [PAPER_REPOSITORIES.md](../../docs/PAPER_REPOSITORIES.md) for
all fields and the publisher contract. `check` now also checks the complete
rendered file list/hashes, including site, archive index and materials; it still
recomputes statistics independently. The render hash manifest is
`.mathesis/rendered.json` and is not itself a source of mathematical verification.

The publisher workflow and fake-API tests live in `mathesis-papers/catalog`.
This SDK remains an anonymous-read-only generator; it never creates repositories,
issues, GitHub commits, Zenodo deposits or DOI registrations.


The public `materials-index.json` sets `generated_at` to the snapshot's `captured_at` and
sorted entries of `kind`, `id`, `version`, `repository_url`, `site_url`, `reading`,
`slides` and `visualizations`. Entries require a durable mapping; mapped papers
without a manifest still link their repository. The file is included in
`.mathesis/rendered.json`. The Mathesis worker optionally consumes it through
`WISHPOOL_MATERIALS_INDEX_URL` and exposes only the current public version's
materials on the existing paper routes.


## Formalization content repository

The renderer also emits `formalization/`, the content for the organization's
`formalization` repository. It is a projection; Mathesis records, independent
verifier receipts and public leaderboards remain the authority. Repository PRs
and CI confer no verification or credit.

Snapshot additionally traverses research-problem pages and public curated
details, downloads confirmed exact target text and all public verified attempts'
bundles, and reads both all-time/all-entrants leaderboards. It validates source
hashes, bundle identity, receipt/toolchain binding and extracted module names
before atomic replacement. `raw/formalization.json` is mandatory for this
renderer; take a new snapshot when upgrading from an older SDK.

The tree has separate `targets/`, `curated/`, `proofs/` and `disproofs/` areas,
`index/nodes.json`, `index/papers/<work>.json`, `index/curated.json`,
`index/modules.json`, both leaderboard representations, build scripts and CI.
Covered/new/open/no-target splits retain public declarations, exact pins and
API reasons. Request-queue URL is defined; write-back request/state/revision
remain null until public authority supplies them. Curator targets are explicitly
not open for submission. Empty source areas still have explanatory READMEs.

`verifier-pins.json` is generated, never edited by hand. After changing verifier
inputs, run `python3 sdk/catalog/scripts/refresh-verifier-pins.py` from the
Mathesis root. It obtains only small dependency locks/config metadata via
anonymous GET and never downloads Mathlib sources. The SDK checks these pins
against the source verifier files, and renders both a complete root Lake
workspace and a Mathlib-only target workspace from them.

From a generated clone, `python3 scripts/check.py` checks integrity without
Lean. Prepare `ci/mathlib` with `lake exe cache get`, then use
`python3 scripts/build.py targets` for compatible targets, separately.
`python3 scripts/build.py <bundle-path> --workspace <prepared-workspace>`
compiles one exact proof/disproof bundle with its recorded library pins. No
aggregate import exists. CI checks integrity and compiles compatible targets;
it does not build the prepared library, compile bundles or run the independent
Mathesis verifier. Historical pin skips and check limits are explicit.

The complete formalization tree is covered by `.mathesis/rendered.json` and
`check`, which independently recomputes its counts/source bindings. See
[FORMALIZATION_REPOSITORY.md](../../docs/FORMALIZATION_REPOSITORY.md) for the
schemas, anonymous image probe, real snapshot, commands, publisher handoff and
the design-only next package splitting SolveFile stores.
