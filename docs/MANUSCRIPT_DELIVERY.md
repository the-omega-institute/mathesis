# Versioned manuscript delivery

Mathesis is the downstream review and publication venue. A producer supplies an
actual PDF/TeX/archive, ordered authors, original problem associations and scoped
evidence. Mathesis reads that source, accepts the submitting account's exact
statement confirmation, runs its ordinary review, and publishes only through its
existing acceptance policy. No upstream verdict, DOI, build badge or Lean URL
substitutes for that decision.

This adapter is not a new common Paper release format. Map the producer's checked
release package into the Mathesis-owned envelope below. The existing
`research-updates.v1` feed belongs to public Pages presentation; it is not a
manuscript or a knowledge-import payload. Trureturing knowledge continues through
the independently admitted, immutable `--knowledge-import` interface documented
in [KNOWLEDGE_OPERATIONS.md](KNOWLEDGE_OPERATIONS.md).

## Delivery and automatic review

Use the authorized submitting account's NyxID identity throughout. The producer
must have the actual author decisions, source rights and processing consent;
the adapter does not invent them. Manuscript authors and their order come from
`paper.authors`, not the uploader's profile or GitHub contributors.

1. POST `/api/v1/manuscript-deliveries` with multipart `metadata` (JSON envelope)
   and `source` (PDF, TeX, ZIP or tar.gz, at most 30 MiB). The JSON alternative is
   `{metadata, filename, source_base64}` using standard base64. Both transports
   store exactly the same file bytes and return the same receipt.
2. Read the returned extracted statements. The producer checks them against its
   claim/evidence map and supplies the author's approved `ClaimConfirmation`
   entries. Confirmation does not need a human editor or a browser session.
3. POST `/api/v1/submissions/{id}/delivery-confirmation` with `version`,
   `source_sha256`, `expected_claims_revision` from that receipt and `claims`.
   The source and statement revision are checked on the same aggregate write.
   Successful confirmation queues the existing review. Exact retries do not
   create a second confirmation or round; stale confirmations return 409.
4. Poll GET `/api/v1/submissions/{id}/delivery` (optional `?version=1`) for the
   small version-bound receipt. Read the final private report through the existing
   `/review.md` or `/review` route only when `review.delivered_at` is present.
   Those report downloads contain the source-version identities and history;
   retain the report for the matching version rather than assuming every included
   round belongs to the newest source.

The CLI and MCP expose the same application services:

```sh
# Existing MATHESIS_URL/MATHESIS_TOKEN or MATHESIS_NYXID_SERVICE configuration.
mathesis-contribute deliver-manuscript delivery.json note.zip
mathesis-contribute confirm-manuscript SUBMISSION confirmation.json
mathesis-contribute manuscript-status SUBMISSION
mathesis-contribute find-manuscript trureturing-paper note-example publication

# A producer that already checked the exact extraction can do both operations.
# claims.json contains an array of approved ClaimConfirmation entries.
mathesis-contribute deliver-manuscript delivery.json note.zip --confirm claims.json
```

The combined command is resumable. It never guesses claims or auto-approves
extraction. A replay of an older delivered version returns `current: false` and
cannot confirm the newer source. The SDK uses the JSON/base64 transport through
NyxID so arbitrary binary source bytes do not pass through a text-only CLI body.
MCP `deliver-manuscript` accepts `metadata`, `filename`, `source_base64`, and
optional approved `confirmations`; `find-manuscript`, `manuscript-status` and
`confirm-manuscript` provide recovery and separate steps.

## Envelope

Example identifiers and URLs below are illustrative, not a real publication:

```json
{
  "schema": "mathesis.manuscript-delivery.v1",
  "producer": "trureturing-paper",
  "work_id": "note-example",
  "version": "1",
  "previous_version": null,
  "paper": {
    "kind": "note",
    "purpose": "publication",
    "title": "A short note on the original question",
    "abstract_text": "The exact result and its scope.",
    "authors": [
      {"name": "First Author", "affiliation": "First institution"},
      {"name": "Second Author", "affiliation": "Second institution"}
    ],
    "processing_consent": "ai-review-2026-10-11",
    "ai_disclosure": {
      "level": "assisted",
      "statement": "Describe the actual assistance used."
    },
    "make_public_after_acceptance": true,
    "open_to_contributors": false,
    "review_scope": "Check the original question, hypotheses, readable proof and evidence scope."
  },
  "origin": {
    "problem_ids": ["original-problem-a", "original-problem-b"],
    "source_url": "https://example.org/research/note-example/v1",
    "license": "CC-BY-4.0",
    "evidence": [
      {
        "claim_label": "thm:main",
        "kind": "written_proof",
        "url": "https://example.org/research/note-example/v1/note.pdf",
        "scope": "Theorem 1 proves the stated parameter range."
      }
    ]
  }
}
```

Use stable producer/work identifiers. One note can name up to 100 explicit problem
IDs; related results do not need separate submissions. `origin.evidence` contains
at most 100 entries. Its claim label must identify a statement actually extracted
from the source, by LaTeX label, display label or extraction ID. Kinds are
`written_proof`, `finite_computation` and `lean_source`. Lean source links carry a
full `source_revision` and point into that exact commit; they remain external
source references, not Mathesis verifier receipts. Keep the precise scope of each
piece of evidence. Necessary files for the reviewers still belong in the existing
[mathesis-evidence.json archive inventory](PROJECT_AND_EVIDENCE_INTAKE.md);
a URL alone does not mean Mathesis fetched or verified its contents.

For a new revision, keep producer/work/purpose, choose a new upstream `version`,
and set `previous_version` to the current Mathesis version. Supply the complete
metadata, source and evidence for that revision. A changed source, authorship or
metadata under an already delivered upstream version returns 409. The adapter
uses actual source bytes and canonical request metadata to detect this conflict.
A newer version cannot be silently overwritten by a delayed request.

Producer/work identity is scoped to the submitting account and purpose. A private
`review_only` manuscript and a `publication` submission are separate explicit
operations, never an implicit conversion. Existing ordinary submissions are not
automatically adopted by a producer identity. Reconcile previously submitted work
before first delivery rather than creating another copy. A DOI is optional and
not part of manuscript identity. Receiving a DOI does not require another source
upload; the producer can keep that citation in its independent publication record.
This endpoint intentionally does not mutate immutable accepted citation snapshots.

## Receipts and recovery

A `mathesis.manuscript-receipt.v1` records the actual submission ID, requested and
current source versions, original upstream version, source digest, confirmation
revision, compile outcome, review summary and matching publication record.

- `next_action: confirm_claims` means a saved draft, not a completed review.
- `status: in_review` and `next_action: wait` mean the ordinary durable pipeline
  is responsible for progress. Retain the same identity while it runs.
- `review.delivered_at` establishes a delivered report for the confirmed version.
  `has_execution_limits` remains explicit. A private report is not acceptance.
- `publication` is present only when the authoritative publication inputs match
  this delivery's version. `publication.public` reflects live visibility.
  Public pages remain pinned to immutable accepted inputs; an unaccepted revision
  never replaces them. Do not infer a new public version from a current status
  label or an old record number.
- `current: false`, `status: superseded` identifies a historical delivery and
  returns any review/publication belonging to it. It does not restart old work.
- Infrastructure interruption remains distinct from mathematical non-acceptance.
  Read the actual report/reasons and use the existing author/staff recovery path.

After a timeout, retry the identical request or GET
`/api/v1/manuscript-deliveries?producer=...&work_id=...&purpose=publication`.
Mongo's unique delivery index prevents concurrent first uploads from creating
multiple submissions. Optimistic revision checks fence updates and confirmations.
Confirmed conflicting creates discard only their own unreferenced candidate blob;
uncertain writes retain evidence. Retrying a persisted delivery repairs missing
compile/review queue admission without starting another round. Existing active
submission limits still apply: the upstream queue should submit a bounded window
and wait for admitted work rather than cycling new work IDs around the limit.

## Public display and ownership

Once accepted and public, the ordinary archive page shows the manuscript authors,
abstract, accepted mathematical statements, PDF, original publication link,
problem associations and scoped evidence links. Private reports, internal
producer keys, source hashes and execution history are not part of that display.
Source associations never add a Lean badge, solver credit or knowledge certificate.
Live privacy and immutable publication inputs govern the projection.

Outreach owns the result queue, reuse of existing papers and returned IDs. Paper
owns writing, PDF/source consistency, claim/evidence matching and package release.
Zenodo and Pages proceed through their own actual publication interfaces. Mathesis
owns this intake, review, acceptance and public presentation. No upstream code,
release format, scheduler, repository credentials or shared CI workflow is changed
by this adapter. A real upstream producer must still call this contract; the API's
existence alone is not evidence that every solved conjecture has a generated note.
