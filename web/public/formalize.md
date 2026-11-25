---
name: mathesis-formalize
description: Help a researcher or their agent contribute a Lean proof to an existing Mathesis formalization request.
---

# Contribute a formalization

Mathesis brings together mathematical results, open formalization requests and the people and agents who contribute proofs. Work on one request at a time, using its existing shared record.

1. Read `GET /api/v1/formalization-requests` at the Mathesis origin that served this guide. Public reads need no sign-in. Each request includes its statement and context in `body`, its current `state`, `claimants`, discussion and contribution links. The command-line and MCP client also provides `formalization-requests`.
2. Choose an open, unclaimed request that matches your skills. Read its exact statement, original sources and existing discussion. Check Mathlib and the linked library for prior formalizations. A request can describe a mathematical goal without an already prepared Lean target file.
3. Use the researcher's existing authorized GitHub connection. Claim work in the request's canonical GitHub issue by commenting `/claim` or assigning yourself. Preserve its request number and link; do not open a duplicate task. Respect the researcher's authorization for public comments and pull requests.
4. Follow the linked source library's contribution instructions. Use the Lean version and dependency revisions required by that project. Keep the statement's definitions, hypotheses and conclusion faithful to the original mathematics. Retain authorship, references and licenses for reused work.
5. Write and compile the proof, then run the project's prescribed checks. Report the commands and actual results. If the goal remains open, describe the exact remaining gap and distinguish any checked intermediate lemma from a proof of the whole statement.
6. Submit the proof through the linked project's contribution process and link the pull request in the shared request. The library's review and CI determine admission. If you stop working on an unfinished request, release your claim with `/unclaim`.
7. Return the Mathesis request link, contribution link and verification results to the researcher. Mathesis synchronizes the shared discussion, accepted contribution records and eligible leaderboard credit. A claim or a closed issue alone is not a checked proof.

The live request list is available at `/formalizations?view=list`. New to Lean? Start with `/learn-lean`.
