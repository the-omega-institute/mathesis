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

# Render the catalog, profile, paper trees and static template.
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
`papers/<repository-name>/` tree per CC-BY-4.0 collected work, and the static
`template/` tree. Accepted author-submitted records are present in the
snapshot and works catalog with `publication: "requires_author_consent"`, but
receive no repository tree.

`src/msc_labels.json` is a copied, package-local projection of the English
MSC labels used by `web/src/data/theory-areas.json` at package creation time.
The SDK does not read `web/` at build time and remains self-contained when
copied elsewhere.

The renderer additionally produces `site/` for `papers.trureturing.org`,
`catalog/archive-index.json` in the exact `mathesis.archive-index.v1` schema,
`catalog/repositories.json`, and `catalog/materials/<work>/materials.json`.
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
