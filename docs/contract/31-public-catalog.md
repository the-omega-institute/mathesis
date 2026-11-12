# §31. Public paper catalog projections

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

- Mathesis public records are the authority. Catalog, profile, paper repositories and `paper-template` are deterministic projections.
- Collection traverses every page of every public listing and every formalization-directory status. A directory/proofs node mismatch aborts collection.
- Snapshots are replaced atomically after complete traversal. A failed traversal leaves the previous snapshot unchanged.
- Rendering reads a snapshot only. It has no wall-clock or map-iteration input.
- A collected node is `collected:<work>:<version>:<claim>`. An accepted-record or other non-collected node is `record:<record>:<version>:<claim>` because the public server has no second accepted-node prefix. Its `lineage` omits the version and keeps the corresponding prefix. History is recorded but not counted.
- Directory rows for accepted records and conjectures remain in `nodes.json`; they do not participate in the collected-work proofs cross-check. Every node has an explicit scope: `in_scope`, `out_of_scope`, or `requires_author_consent`. Nodes outside publication scope are excluded from every profile count.
- A Lean source link at an exact commit does not establish independent verification. Verification requires an exact target and a public verifier receipt.
- Verified nodes retain their library-declaration or new-Lean basis. Unverified nodes have no basis.
- Phase 1 renders trees only for collected `CC-BY-4.0` works. Accepted author-submitted records require consent and receive no tree. Rendering a tree does not publish it; which trees become repositories is the publisher's decision. The catalog still records their `concept_doi` and other public identity fields when supplied.
- The generator performs anonymous GET requests only. It never registers a DOI or writes GitHub or Zenodo.
- Frozen files remain byte-identical. Supplied public SHA-256 values are enforced. Archives reject traversal, links, excessive entries and excessive unpacked size.
- Pull requests do not confer formalization verification or credit. Formalization attempts use the Mathesis attempt path and independent verifier.
- `check` recomputes `stats.json` from `works.json` and `nodes.json` and exits non-zero on a difference.
- `paper.json` records `files.pdf` and `files.source` with `path`, `bytes`, `sha256`, `public_sha256`, and nullable `matches_public`; statement mappings live only in `formalization/nodes.json`.
- Profile links use the organization `catalog` repository and the site routes `/papers/<record>?version=<version>`, `/formalizations`, and `/contribute`.
