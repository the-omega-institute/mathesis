# §18. Versioned downstream manuscript delivery

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

- `/manuscript-deliveries` consumes actual paper/note sources and an explicit
  `mathesis.manuscript-delivery.v1` envelope. Stable owner/producer/work/purpose
  identities have a partial unique Mongo index; each upstream version retains
  its request digest, actual source bytes and source associations. Exact retries
  recover the same submission; changed payloads under one version are refused.
- Revisions name their current predecessor. Delivery confirmation fences source
  version, actual digest and statement revision in the same ordinary author
  confirmation write. Its saved receipt makes retries idempotent and repairs
  queue admission. No automatic guessed statements, consent or upstream verdict.
- Private review and publication are distinct identities. Current delivery,
  delivered report, execution limits and version-matched public record are
  independent receipt fields. Prior accepted inputs remain immutable and private
  revisions cannot leak through origin/evidence fields.
- Public source associations are projected only from accepted public inputs.
  External Lean links confer no verifier badge or credit. Knowledge import is
  unchanged. See `docs/MANUSCRIPT_DELIVERY.md` for API, CLI/MCP and producer handoff.
