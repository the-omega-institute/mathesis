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

## Follow-up after the fourth review pass

Applied in a later commit on the same branch, with the gates rerun (429 tests):

- The cross-surface badge/headline fixture is keyed by the `SubmissionStatus`
  union (`authorStandingCasesByState`), so a new state cannot be left out.
- The raw-identifier sweep also runs on draft and withdrawn pages and reads
  `aria-label` attributes.
- My work rows show the work kind in the kind slot; the standing headline
  carries "Private review …" alone. Rows fetch their referee file once and
  never poll (`useReferee(…, poll = false)`); a focus event starts no refresh.
- The AI-use radio group carries the HTML `required` attribute.
- One author wording for stage states (`AUTHOR_STEP_STATE_LABELS`) feeds the
  progress list and the compute history.
- Author names read from `\author{}` are shown as text: TeX accents and
  ligatures (`\ae`, `\'e`, `\H{o}`, `\c{c}` …) become Unicode on every author
  surface (`web/src/lib/texText.ts`), found when previewing the signed-in pages
  locally with the arXiv test note.

Verified in a local preview (dev sign-in, memory storage, no TeX): the Submit
form, the compile-failure standing card with its "Upload a corrected version"
control, and the My work row.

- The details mount of `RefereeWorkspace` no longer carries its own loading
  and error shell: the primary mount owns it for the shared referee resource,
  and the details render once the same file has loaded (one "Loading…" on the
  author page, tested).

## Remaining advisories (not blocking)

- Two moderate npm audit advisories in the vitest dev dependency predate this
  change.

## Conflict to resolve before merge

The main working tree held an uncommitted web redesign dated 17 October 2026
(43 modified files plus nine new ones) on no branch or PR. This change does
not build on it. The overlap, measured read-only on 22 October:

| 2026-10-17 tree | This branch | Note |
|---|---|---|
| `web/src/lib/authorReview.ts` (status label/description/tone helper) | `web/src/lib/authorStanding.ts` | same purpose; this branch's version also gives next action, elapsed time, dated observation and feeds badge, card and rows |
| `FinalReviewReport.tsx`, `ReviewOverview.tsx`, `WorkspaceTabs.tsx` (new) | outcome-first ordering inside `referee.tsx` and `SubmissionDetail.tsx` | different decomposition of the same page |
| `formalizationStatus.ts` (new) | not covered | Lean status helper; could be carried over on its own |
| 21 shared files incl. `SubmissionDetail.tsx`, `referee.tsx`, `reviewProgress.ts`, `labels.ts`, `Papers.tsx`, `layout.css` | modified here | textual merge conflicts certain |

Decision for the owner: keep this branch (reviewed, tested, deployed) and
port `formalizationStatus.ts` if wanted, or keep the 17 October tree and
re-apply the reviewed behaviours. Doing both requires a manual merge of the
21 shared files.
