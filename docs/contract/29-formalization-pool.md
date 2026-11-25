# §29. Shared formalization participation

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

The formalization list and Lean contribution entry read the shared request
snapshot. Users can filter open, claimed and merged requests and read each
request's Markdown, mathematical notation, discussion and contribution evidence
inside Mathesis. Task titles and leaderboard request numbers use stable in-site
links. The original GitHub issue is a secondary source link; GitHub remains the
authority for participation and Mathesis does not create a competing queue.
Repository-wide paginated comment reads keep synchronization within the public
API budget instead of making one request per discussion. Refreshes backfill old
snapshots' missing descriptions and comments, and failed
refreshes retain the last complete snapshot. GitHub assignees and exact self-authored `/claim`
comments are claims; `/unclaim` or `/release` releases a comment claim, without
overriding assignment. Mentions, closure and claims alone earn no points. The
formalization leaderboard uses eligible linked merged contributions, deduplicated
per contributor/request, with completion-month filtering. It does not derive
new Mathesis kernel receipts or add to the historical local proof leaderboard.

The subject DAG retains the published formalized library. Open/claimed shared
requests add hollow nodes, located by their exact linked source module where
available; otherwise their subject stays unclassified. Context links are not
proof dependencies. Neither a closed request nor a merged PR alone creates a
solid verified library node. A missing request snapshot must not hide the
available mathematical library. Historical confirmed-target/receipt APIs remain
readable. The four public views are the existing DAG, awaiting formalization,
the contribution leaderboard and the Lean learning guide.
