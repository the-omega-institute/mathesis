# §14. Mathesis identity and source projects

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

- Mathesis is the public product and repository name. Follow
  `docs/MATHESIS_MIGRATION.md` for compatibility: persistent WP ids, proof names,
  identity subjects, database names, cookies and WISHPOOL server config remain.
- Formalization is a primary participation path. The public directory lists
  current public collected results awaiting formalization, confirmed public targets
  and independent verifier results;
  private deposits, queued/rejected attempts and stale targets are excluded.
- Deposits may include 512 dotted-path supporting Lean modules, at most 8 MiB together
  with the optional Solution adapter (1 MiB maximum). Inert project configuration
  is preserved separately (16 allowlisted root files, 1 MiB total). They are immutable GridFS sources. Record original authors,
  formalizers and optional full Git revision separately from uploader identity.
- Project verification uses the same independent checker and durable attempts.
  Scan every module, reject cycles, unsafe constructs and reserved/shadowing
  names, compile in dependency order, and bind all sources into the receipt.
  Do not execute user Lake scripts or install custom dependencies. Only the
  existing pinned workspace supplies dependencies. Single-file hashes stay stable.

- Complete projects can be source-only deposits. Preflight reports missing imports,
  cycles, dependency/target pin conflicts, and restricted-profile constructs.
  Verification may attach a separate immutable Solution adapter; saved modules stay intact.
  Supporting modules may contain namespace/section blocks and Mathlib submodule imports;
  each Mathlib import must resolve to a prepared artifact before Lean runs. Root target
  and adapter scans remain strict, including comments. Preflight is not a receipt.
  Supporting modules may contain issue-number hashes in line or nested block
  comments. Modules with quoted literals retain the conservative hash rule to avoid
  confusing Lean interpolation with comments. Hash commands remain forbidden; all other
  restricted-construct, import, target, axiom and provenance checks are unchanged.
- Manuscript archives may declare `mathesis-evidence.json` schema 1: up to 100
  relative-path attachments with kind, description, optional claims and full Git revision.
  Derive byte counts and SHA-256 from actual archive files; bind inventory to PaperVersion.
  Source access rules also protect attachment downloads. No attachment is a proof receipt.
  Review export includes inventories by version. Oracle gets up to 64 KiB per UTF-8
  attachment / 256 KiB total; CMA gets a bounded source inventory and up to 512 KiB per
  UTF-8 file / 2 MiB total. Explicitly identify every inventory-only file; never claim
  it was read or validated. Uploaded scripts are not automatically executed.

- See `docs/REVIEW_FOLLOWUP.md` for version-bound response assessments and asynchronous
  CMA cleanup. A stop acknowledgment does not imply that an agent can yet be deleted.

- Keep legacy review prompt bytes unchanged when no computational evidence is declared;
  existing admitted task input digests must remain resumable across this upgrade.
