# Author experience as a new-era arXiv, 22 October 2026

Record of the web-only change on branch `claude/arxiv-experience` and of the
multi-perspective review that produced it. Policy, consent, statement
confirmation, the review pipeline and every API route are unchanged.

## Goal

A working mathematician with no AI or software background should be able to
use Mathesis the way they use arXiv: upload a LaTeX manuscript with the minimum
required inputs; always know what state the submission is in, what Mathesis is
doing, what to do next and how long it took on this deployment; read the
decision and the letter as the primary content when they exist; browse accepted
work through listing and abstract pages. Legibility follows the CMA/NyxID
resource model: one object, one status vocabulary, one next action per state,
stable identifiers, no internal mechanism names on author surfaces.

## What changed (web/ only)

- **Submit**: the three required inputs for a paper or note (LaTeX source,
  AI-use choice, processing consent) come first, each with one sentence on why
  the venue asks; everything else is grouped as optional; a short "what happens
  after you upload" primer; typed conjectures mark title, statement and author
  as required with the same kind of sentence (Layer 2 requires a nonempty
  author list).
- **One author standing** (`web/src/lib/authorStanding.ts`): a single
  derivation from the submission and the author-safe referee file gives, for
  every state, what Mathesis is doing, what the author does now or "Nothing to
  do now", elapsed time since the current stage started and a dated observation
  of stage durations measured on this deployment on 22 October 2026 for a
  four-page note (PDF under 1 min; referee report 19 min; source check 8 min;
  improvement suggestions 9 min; feedback letter 2 min; about 45 min upload to
  letter; totals vary with queue and retries; never a promise). The submission
  page badge, the standing card and the My work rows all show that headline.
  One stage vocabulary everywhere: Referee report, Source check, Improvement
  suggestions, Feedback letter, Lean check. Failure copy never tells authors to
  "try again" (restart is staff-only); a failed step is not a verdict on the
  mathematics; a corrected version may be uploaded, and the compile-failure card
  opens the upload form directly. "Review ready" only when the current round's
  letter has been delivered. Without a current round the page says progress
  details are unavailable instead of inventing queued work.
- **Outcome first**: once a decision or letter exists the page reads standing
  (decision, record id, reasons) → current delivered letter → readable report →
  statements → one closed "Details" section holding evidence, history,
  advanced compute settings and identifiers. Author surfaces show no raw review
  method, engine or model ids, payer, workspace ids, or enum values such as
  `bind_only`/`not_checked` without a plain label. Findings carry status labels
  (Confirmed, Refuted, Further evidence needed); authors may respond to any
  finding, as Layer 2 admits; only confirmed findings need a response.
- **Public archive**: listing cards with an abstract excerpt (full LaTeX render,
  four-line clamp); paper page with version history and per-version PDF links,
  DOI and a derived arXiv label for `10.48550/arXiv.*` DOIs.
- **Errors**: a network failure says the request could not be confirmed and
  that reloading after reconnecting shows the current state; it neither claims
  the server is unchanged nor says "try again".

## How it was produced

Six independent thinking seats (four Codex workers, one GPT Pro oracle, one
isolated subagent) proposed plans; the caller converged them into one web-only
plan (a Layer 2 "author standing" projection was recorded as a later option,
not needed while every author-safe fact is already exposed). One
implementation worker built it; three independent reviewers (architecture,
quality, tests; rotating carriers) rejected three times and approved on the
fourth pass after three bounded repair batches. Gates on the final tree:
`format:check`, `lint`, `typecheck`, `test` (426 tests), `build`,
`build:showcase`, `npm audit --audit-level=high` all pass; only `web/` changed.

## Remaining advisories (not blocking)

- The cross-surface badge/headline fixture is a plain array, not tied to the
  `SubmissionStatus` union; the identifier sweep does not cover draft or
  withdrawn pages or `aria-label` attributes.
- My work rows for private reviews read "Private review · Private review in
  progress"; the kind slot could be dropped there.
- Each non-draft My work row fetches its referee file (N requests per page);
  a list-level projection would bound it.
- The AI-use radio group is marked required but carries no HTML `required`
  attribute (the default `none` is a valid choice).
- Three step-state word tables still coexist for staff and history views.
- Two `RefereeWorkspace` mounts on the submission page each carry their own
  loading shell; two author components receiving the loaded file would be
  cleaner.
- Two moderate npm audit advisories in the vitest dev dependency predate this
  change.

## Conflict to resolve before merge

The main working tree held an uncommitted web redesign dated 17 October 2026
(44 files, including a rewrite of the submission page and an `authorReview`
helper) on no branch or PR. This change does not build on it. The owner
decides which lineage to keep; merging both will conflict in
`SubmissionDetail.tsx`, `referee.tsx`, `reviewProgress.ts` and styles.
