# §13. Research service intake and discovery

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

- The archive has MongoDB text/metadata indexes. Filtered search uses immutable
  publication inputs and rechecks live visibility before returning results.
  Search must never match private drafts or private review text.
- `/submissions/{id}/review` exports a version/digest-bound Orchestra dossier
  using the author-safe referee projection, even for staff. Authors can see
  available progress, reports and audits before the delivered letter.
- `/formalizations` receives existing Lean modules as private, attributed
  deposits, with original authors, source reference, pinned toolchain and license.
  Sources live in GridFS; metadata lives in `lean_deposits`; exact retries are
  idempotent. Receiving a source is never proof verification or paper acceptance.
  Concurrent retries discard only their own fresh source blobs after a confirmed
  conflicting deposit is read, preserving every blob it references. Cleanup is
  best effort and never runs after an uncertain insert outcome.
- Deposit owners may explicitly request verification against an existing public,
  author-confirmed target. Reuse the ordinary attempt and independent verifier;
  link attempts under a revision fence. No separate acceptance route, arbitrary
  project execution or editor verification is introduced.
- User-facing services are Mathesis review and community formalization.
  CMA account configuration belongs in advanced compute settings. NyxID remains
  the identity and connection provider. API, OpenAPI, CLI and MCP project the
  same Layer 2 services. See `docs/RESEARCH_SERVICES.md` for exact current scope.

- Audited concerns retain reported location, evidence and basis separately, including
  offline calculations. A reported source is not an authenticated citation.
- Authors/co-authors may append immutable responses to an audited finding, bound to
  the current confirmed source version and claims revision. The ordinary durable
  referee queue rechecks them. Exact retries are idempotent; at most five responses
  per paper per 24 hours, 200 total. Replies never resolve findings automatically.
  Queue reconciliation recovers a saved response after enqueue failure.
- Review JSON schema 2 and `/review.md` include each round's source digest and author
  responses. Both use the author-safe view, including when requested by staff.
- New rounds pin review contract 3. Contract 1 introduced re-review assessments;
  contract 2 additionally carries bounded attachment contents and their actual
  version-bound hashes into audit, advice and letter prompts. Earlier rounds
  retain their original prompts, including when evidence was not transported.
  Contract 3 adds the structured Trureturing methodology above. Historical
  reports and decisions retain their original meaning; no automatic backfill.
  Re-review audits assess each admitted author
  response as addressed, unresolved or not_checkable, with a concrete explanation.
  Attachment references resolve to paths and actual SHA-256 digests in that round's
  source version. Unknown/duplicate response outputs or unusable evidence cannot
  establish addressed. Missing model conclusions become explicitly not_checkable.
  These are reviewer judgments, never execution or Lean receipts; old findings stay
  immutable. Legacy rounds retain their admitted prompt bytes and receive no fabricated
  retrospective conclusions.
- Public verified attempts offer a JSON verification bundle: exact confirmed target,
  solution/modules, pinned toolchain, attribution, source/claim identity and receipt.
  Current public visibility and target validity are checked on export. Rechecking
  requires a prepared matching Lean/Mathlib workspace; no historical checker binary
  revision is invented and no private probe files are exported.
