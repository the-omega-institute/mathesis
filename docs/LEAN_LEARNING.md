# Lean learning resources

`/learn-lean` is the public, bilingual learning guide. It introduces existing Lean
4 resources in a practical order: browser exercises, mathematical proofs, Mathlib
reuse, community help and a first Mathesis contribution. It does not require an
account. The Formalizations tabs, contribution guide and site footer link to it;
the main navigation remains focused on research and submission.

The introduction explains Lean before recommending tutorials: it is free,
open-source software, a programming language and a proof assistant. It describes
formalization and reuse, gives a complete browser example, and explains VS Code
checking and `lake build`. Build success is distinguished from a gap-free proof;
configured build targets need not include every file. The relation between proofs
and programs does not mean every theorem automatically yields an executable
algorithm. Constructive proofs can describe algorithms with verified properties.

The resource descriptions explain audience and use, and identify the resource
language independently of Mathesis's interface language. Original titles and
community attribution are preserved. A Mathlib contribution and a Mathesis proof
contribution have different review processes; the page links to each explicitly.

Sources checked on 2026-11-10:

- Lean overview: <https://lean-lang.org/>.
- Proofs and programs: <https://lean-lang.org/theorem_proving_in_lean4/propositions_and_proofs.html>.

- Lean's official learning directory: <https://lean-lang.org/learn/>. It recommends
  the Natural Number Game for beginners and Mathematics in Lean for mathematicians.
- Natural Number Game: <https://adam.math.hhu.de/#/g/leanprover-community/NNG4>.
  Preserve the uppercase `NNG4` game identifier used by the official directory.
- Lean Web: <https://live.lean-lang.org/>.
- Mathematics in Lean: <https://leanprover-community.github.io/mathematics_in_lean/>.
  Its companion project is <https://github.com/leanprover-community/mathematics_in_lean>.
- Theorem Proving in Lean 4: <https://lean-lang.org/theorem_proving_in_lean4/>.
- Official installation: <https://lean-lang.org/install/>.
- Mathlib documentation: <https://leanprover-community.github.io/mathlib4_docs/>.
- Loogle: <https://loogle.lean-lang.org/>.
- Existing projects: <https://leanprover-community.github.io/install/project.html>.
- Lean Zulip: <https://leanprover.zulipchat.com/>.
- Mathlib contributions: <https://leanprover-community.github.io/contribute/index.html>.

Recheck the destination and substantive description when updating a resource.
Do not link a Lean 3 tutorial as Lean 4, imply that all external resources are
translated, or describe a linked source file as a new independently verified
Mathesis proof. Keep the guide a readable resource directory rather than copying
external textbooks or adding a separate proof execution service.
