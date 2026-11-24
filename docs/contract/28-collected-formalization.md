# §28. Shared formalization execution

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

`the-omega-institute/trureturing-requests` is the canonical work queue for people
and agents. Mathesis consumes its requests, explicit claims and linked merged
Trureturing contributions; it does not maintain an independent collected-paper
CMA proof executor or reserve a dedicated proof sandbox. Collected source
indexing may retain statement discovery records, but must not enqueue
`collected_formalization`. Startup supersedes waiting, failed and expired leased
jobs of that obsolete kind, preserving identities, history and completed receipts.
An active lease is left to its holder/expiry. Review and exposition jobs are not
retired by this migration.

Issue closure is not a proof. Completion requires a merged Trureturing PR with
an exact back-reference to the request, a full merge revision and merge time.
That incorporation record is distinct from an independent Mathesis verifier
receipt. Preserve all historical target, proof, verifier and attribution records.
Competitive ranking requires an explicitly verified external GitHub identity in
`WISHPOOL_REQUESTS_RANKED_LOGINS` and excludes known platform accounts and
requester self-work; configure all verified venue/author GitHub identities in
`WISHPOOL_REQUESTS_UNRANKED_LOGINS`. A GitHub login is not proof of a human's
identity: unresolved author/coauthor identity must be checked before awarding
competitive credit. External contributions to our papers remain eligible.

Synchronization is a separate bounded, read-only worker. Replace only complete
snapshots under revision fencing, and retain the last good snapshot on transport,
rate-limit or validation failure. Operator bootstrap uses the same Rust adapter
and an administrator-only import route; no publication or GitHub write is made.
