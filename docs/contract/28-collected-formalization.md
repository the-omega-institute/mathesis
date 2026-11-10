# §28. Automatic collected-paper formalization

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

Internal collected publications automatically enter one durable proof job per
current paper version. Reconciliation backfills historical indexed results and
repairs interrupted admission. No manuscript re-review or Oracle is involved.
New jobs read the independent verifier's ready `/environment`, pin the exact
Lean/Mathlib pair before provider admission, and instruct CMA to reuse or prepare
a matching version-specific environment with a compiled-cache/import check.
Keep old environments and exact prompt bytes for already admitted work. Never
upgrade a shared proof directory in place or rewrite targets and receipts.
An optional legacy verifier URL routes historical pins to the prior service.
CMA outages retain the original job and remote identities without spending a
proof generation. A second semantic assessment admits exact translations;
independent kernel verification alone grants checked status and proof credit.
New jobs also pin the verifier's prepared libraries with the environment and,
when any is declared, work library first: prove the Mathlib-only target from an
existing library declaration, and port or newly prove only what the library lacks.
