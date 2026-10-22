# First complete automatic review on chrono-cd, 22 October 2026

Record of the first end-to-end review on the Omega Research deployment
(<https://mathesis-omega-research.apps.chrono-ai.fun>, chrono-cd generation 9,
revision 12, image `ghcr.io/the-omega-institute/mathesis-api:cma-3809ef8`, which
carries [CMA_DEPLOYMENT_2026-10-22.md](CMA_DEPLOYMENT_2026-10-22.md) and
[CMA_ADMISSION_RETRY_2026-10-22.md](CMA_ADMISSION_RETRY_2026-10-22.md)).
Identifiers only; no credentials.

## Input

A public four-page arXiv note (arXiv:2608.20431, "A note on least primitive
roots"), uploaded through `POST /api/v1/submissions` with a NyxID user access
token as `purpose: review_only`, `kind: note`, private, AI use `none`,
consent `ai-review-2026-10-11`. Submission `01a164ac-a7c5-776a-b735-93fba2c4babc`.
Four statements were extracted (main theorem, a proposition with its proof, a
cited theorem, a corollary) and confirmed with their dependency edges. The
first confirmation attempt that excluded the cited theorem was refused with
`invalid input: C2 depends on excluded statement C3`; keeping it as a
supporting statement was accepted.

## Observed timeline (UTC)

| Step | Start | End | Duration |
|---|---|---|---|
| Upload, source read, PDF build, statement extraction | 17:33:19 | 17:33:3x | under 1 min |
| Author confirmation → round 1 started | 17:34:00 | 17:34:xx | immediate |
| Knowledge queries (contract 3) | 17:34 | 17:35 | under 1 min |
| GPT Pro referee (`gpt_6 · pro`) | 17:34 | 17:53:10 | 19 min |
| CMA audit (`r1:audit`, one attempt, succeeded) | 17:56:11 | 18:04:16 | 8 min |
| Status `review_complete` | | 18:06 | |
| CMA advice (`r1:advice`) | 18:05:18 | 18:14:25 | 9 min |
| CMA letter (`r1:letter`) | 18:15:28 | 18:16:51 | 1.5 min |
| Letter delivered in-app | | 18:16:52 | |

Upload to delivered letter: 43 minutes. The earlier smoke test with a
one-line theorem took 7 minutes for the referee and 4 minutes for the audit;
those numbers do not transfer to a real manuscript.

## Outcome

Referee recommendation `major_revision`; the audit and letter identify three
concrete issues in the proof and a plausible revision route. The letter opens
with "Your private Orchestra review is complete. This is research feedback,
not a publication decision." No publication decision, record or Lean probe
exists for a review-only submission, as designed. The CMA agents of all three
steps were deleted after their answers.

## Still outside this test

- A publication-mode round on this deployment, which additionally needs an
  admitted Trureturing knowledge index for contract 3.
- Browser sign-in and the author-facing pages (exercised through the API only).
