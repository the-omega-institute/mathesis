# §31. Public paper catalog projections

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

- Mathesis public records are the authority. Catalog, profile, paper repositories and `paper-template` are deterministic projections.
- Collection traverses every page of every public listing and every formalization-directory status. A directory/proofs node mismatch aborts collection.
- Snapshots are replaced atomically after complete traversal. A failed traversal leaves the previous snapshot unchanged.
- Rendering reads a snapshot, explicit mapping and optional local repository manifests only. It has no wall-clock or map-iteration input.
- A collected node is `collected:<work>:<version>:<claim>`. An accepted-record or other non-collected node is `record:<record>:<version>:<claim>` because the public server has no second accepted-node prefix. Its `lineage` omits the version and keeps the corresponding prefix. History is recorded but not counted.
- Directory rows for accepted records and conjectures remain in `nodes.json`; they do not participate in the collected-work proofs cross-check. Every node has an explicit scope: `in_scope`, `out_of_scope`, or `requires_author_consent`. Nodes outside publication scope are excluded from every profile count.
- A Lean source link at an exact commit does not establish independent verification. Verification requires an exact target and a public verifier receipt.
- Verified nodes retain their library-declaration or new-Lean basis. Unverified nodes have no basis.
- Render trees for public collected `CC-BY-4.0` works and accepted author papers whose public archive carries the exact consent marker, positive published version, licence, consent time and hashed frozen files. A listing's licence or consent boolean alone never authorizes copying; fetch and compare its current public detail before snapshot completion. Legacy, private and unconsented records receive no tree or material links. A changed public detail aborts the atomic snapshot. Rendering does not publish; repository selection is a reviewed publisher policy.
- The generator performs anonymous GET requests only. It never registers a DOI or writes GitHub or Zenodo.
- Frozen files remain byte-identical. Supplied public SHA-256 values are enforced. Archives reject traversal, links, excessive entries and excessive unpacked size.
- Pull requests do not confer formalization verification or credit. Formalization attempts use the Mathesis attempt path and independent verifier.
- `check` recomputes `stats.json` from `works.json` and `nodes.json` and exits non-zero on a difference.
- `paper.json` records `files.pdf` and `files.source` with `path`, `bytes`, `sha256`, `public_sha256`, and nullable `matches_public`; retain the original source bytes as `source.archive` alongside the extracted manuscript. Accepted uploads may be archives, a single TeX file or a PDF; no submitted code runs. Statement mappings live only in `formalization/nodes.json`.
- Profile links use the organization `catalog` repository and the site routes `/papers/<record>?version=<version>`, `/formalizations`, and `/contribute`.
- The org site is an offline projection at `papers.trureturing.org`, with scoped counts and snapshot time. Published repository/site links require the durable mapping. Synchronization preserves `CNAME` and unrelated files.
- `archive-index.json` uses exactly `mathesis.archive-index.v1` (PR #120): snapshot RFC 3339 `generated_at`, `entries` containing `kind`, `id`, positive `version`, exact-version `doi`. Concept DOIs never substitute for version DOIs; the server independently verifies archive bytes and consent.
- Every work has `materials/<work>/materials.json`. Repository manifests use `mathesis.materials.v1`; catalog projections use `mathesis.catalog-materials.v1`. Material identity/version/site boundaries are checked offline. Unavailable outputs stay absent, and DOI authority stays with Mathesis. Server material consumption follows §33.
- The external publisher may create repositories for in-scope works selected by its reviewed policy (explicit allow-list or automatic public archive eligibility). It fills/checks skeletons or publishes existing collected materials independently of model availability; it does not generate reading, media, receipts or verification status. Per-paper generated identities depend on their content, not the wall-clock snapshot time, so an unchanged paper needs no new publication commit.
- Snapshot/fetch/render/check and all skeleton preflight checks must finish before GitHub writes. Staging failures publish nothing. Apply is atomic per repository, never force-pushes, and resumes partial repository/Pages/issue setup on retry. The App key stays in the catalog repository; the server receives no publisher credentials.
- `check` verifies the rendered file inventory and SHA-256 values in addition to independently recomputing statistics.
