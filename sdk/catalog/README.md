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
