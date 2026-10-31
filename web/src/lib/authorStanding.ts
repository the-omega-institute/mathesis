import type { RefereeFile, RefereeRound, StageProgress, Submission } from '../api/types';
import { SUBMISSION_STATE_LABELS } from './labels';
import { hasWaitingResponse, REVIEW_STAGES, reviewDelivered } from './reviewProgress';

export const TIMING_OBSERVATION =
  'Observed on this deployment on 22 Oct 2026 for a four-page note: PDF under 1 min; Referee report 19 min; Source check 8 min; Improvement suggestions 9 min; Feedback letter 2 min; about 45 min from upload to letter. Totals vary with the queue and retries; this is not a promise or deadline.';
export const SAVED_FAILURE =
  'Your upload and any report are saved. A failed step is not a verdict on the mathematics. A corrected version may be uploaded.';

export function awaitingDecision(s: Submission, delivered: boolean): boolean {
  return s.status.state === 'in_review' && s.purpose !== 'review_only' && delivered;
}

export function decisionWaitingMessage(s: Submission): string {
  return s.decision?.decision === 'pending' && s.decision.awaiting === 'literature'
    ? 'The review analysis is complete. A comparison with prior work is still needed before a publication decision.'
    : 'The review analysis is complete. The publication decision is pending.';
}

export function currentAuthorRound(s: Submission, file?: RefereeFile): RefereeRound | undefined {
  return file?.rounds
    .filter(
      (r) => r.version === s.versions.at(-1)?.number && r.claims_revision === s.claims_revision,
    )
    .reduce<RefereeRound | undefined>(
      (latest, r) => (!latest || r.number > latest.number ? r : latest),
      undefined,
    );
}

function elapsed(since: string, now: number) {
  const minutes = Math.floor(Math.max(0, now - Date.parse(since)) / 60_000);
  if (!Number.isFinite(minutes)) return 'unavailable';
  if (minutes < 1) return 'less than 1 min';
  if (minutes < 60) return `${minutes} min`;
  return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}

/** The author vocabulary and next action, shared by the workspace and My work. */
export function authorStanding(s: Submission, file?: RefereeFile, now = Date.now()) {
  const round = currentAuthorRound(s, file);
  const delivered = !!round && reviewDelivered(round, file?.letters ?? []);
  const waiting = hasWaitingResponse(file, s);
  const version = s.versions.at(-1);
  let headline = SUBMISSION_STATE_LABELS[s.status.state];
  let doing =
    'Progress details are loading or unavailable; the review is in progress on the server.';
  let nextAction = 'Nothing to do now. You can leave this page and return later.';
  let since = round?.started_at ?? s.updated_at;
  let elapsedLabel = round
    ? 'Elapsed since this review started'
    : 'Elapsed since the last submission update';
  let stage: (typeof REVIEW_STAGES)[number][0] | undefined;

  if (s.status.state === 'draft') {
    since = version?.uploaded_at ?? s.created_at;
    elapsedLabel = 'Elapsed since this version was uploaded';
    headline = 'Ready for your check';
    doing = 'Your manuscript is ready. Check the PDF, then start the review.';
    nextAction = 'Check the manuscript PDF and start the review.';
    if (version?.compile_error) {
      headline = 'PDF could not compile';
      doing = `Mathesis could not compile the PDF. ${SAVED_FAILURE}`;
      nextAction = 'Check the compilation details; you may upload a corrected version.';
    } else if (version && !version.pdf) {
      headline = 'PDF compiling';
      doing = 'Mathesis is compiling the PDF and preparing the statement list.';
      nextAction = 'Check the manuscript PDF when it is available.';
    }
  } else if (s.status.state === 'withdrawn') {
    headline = 'Withdrawn';
    doing = 'Mathesis has closed this submission after withdrawal.';
  } else {
    if (round) {
      const stages = delivered ? REVIEW_STAGES.slice(4) : REVIEW_STAGES.slice(0, 4);
      // Safe progress includes hidden stages. Old files expose only visible steps;
      // absence of a private draft never establishes completion or delivery.
      for (const [key, label] of stages) {
        if (key === 'formal' && (s.status.state !== 'accepted' || s.kind === 'conjecture'))
          continue;
        const step: StageProgress | undefined = round.progress?.[key] ?? round[key]?.state;
        if (!step || step.state === 'done' || step.state === 'skipped') continue;
        stage = key;
        if (step.state === 'running') {
          since = step.since;
          elapsedLabel = `Elapsed since ${label} started`;
          doing =
            key === 'formal'
              ? 'A formalization attempt is in progress.'
              : 'Mathesis is reviewing your manuscript.';
        } else if (step.state === 'pending') {
          const preceding = REVIEW_STAGES.slice(
            0,
            REVIEW_STAGES.findIndex(([k]) => k === key),
          )
            .map(([k]) => round.progress?.[k] ?? round[k]?.state)
            .reverse()
            .find((p) => p?.state === 'done');
          since = preceding?.state === 'done' ? preceding.at : round.started_at;
          elapsedLabel = `Elapsed waiting for ${label}`;
          doing =
            key === 'formal'
              ? 'A formalization attempt is queued.'
              : 'Your review is queued and will continue automatically.';
        } else {
          headline = 'Review could not finish';
          doing = `${label} needs attention. ${SAVED_FAILURE}`;
          nextAction =
            'The manuscript is saved. Staff can inspect and recover the interrupted review.';
          // A failure has an end time, but older projections do not retain its start.
          elapsedLabel = 'Elapsed since this review started (step start time is unavailable)';
        }
        break;
      }
      if (!stage) {
        doing = delivered
          ? 'Your final review report is available below.'
          : 'The available review steps are complete. No feedback letter has been delivered for this round.';
      }
    }
    if (!round && s.status.state !== 'in_review' && !waiting) {
      doing =
        'Progress details for this version are unavailable. Read any available Feedback letters, reports and review history in Details.';
      nextAction =
        s.status.state === 'accepted'
          ? 'Nothing is required. You can read the available letters and reports.'
          : s.status.state === 'not_accepted'
            ? 'You may upload a corrected version after reading the available letters and reports.'
            : 'Read the available report, Feedback letters and review history in Details.';
    }
    if (s.status.state === 'review_complete') {
      if (delivered) headline = 'Review ready';
      else if (!round && !waiting) headline = 'Review progress unavailable';
      else if (headline !== 'Review could not finish') headline = 'Private review in progress';
    } else if (s.status.state === 'accepted')
      headline = s.kind === 'conjecture' ? 'Displayed' : 'Accepted';
    else if (s.status.state === 'not_accepted')
      headline = s.kind === 'conjecture' ? 'Not displayed' : 'Not accepted';

    if (delivered && !stage) {
      const letter = file?.letters
        .filter((l) => l.round === round?.number)
        .sort((a, b) => Date.parse(b.sent_at) - Date.parse(a.sent_at))[0];
      since = letter?.sent_at ?? since;
      elapsedLabel = 'Elapsed since the current Feedback letter arrived';
    }
    if (delivered && nextAction.startsWith('Nothing'))
      nextAction = 'Read the analysis and suggestions in your report.';
    if (waiting) {
      stage = undefined;
      const response = file?.finding_responses
        ?.filter((r) => r.version === version?.number && r.claims_revision === s.claims_revision)
        .sort((a, b) => Date.parse(b.at) - Date.parse(a.at))[0];
      since = response?.at ?? s.updated_at;
      elapsedLabel = 'Elapsed since your response was saved';
      doing = 'Your response is saved. Mathesis is waiting to start a follow-up review.';
      nextAction = 'Nothing to do now. The follow-up review will check your response.';
      if (s.status.state !== 'accepted' && s.status.state !== 'not_accepted')
        headline = 'Your response is awaiting review';
    } else if (
      round?.response_ids?.length &&
      !delivered &&
      headline === 'Private review in progress'
    )
      headline = 'Your response is being reviewed';
  }
  if (awaitingDecision(s, delivered)) {
    headline = 'Awaiting decision';
    doing = decisionWaitingMessage(s);
    nextAction = 'No action is needed from you.';
  }
  const waitingForService =
    !delivered && !!round?.waiting_since && headline !== 'Review could not finish';
  if (waitingForService) {
    if (!['accepted', 'not_accepted'].includes(s.status.state))
      headline = 'Review temporarily delayed';
    doing =
      'The review service is temporarily unavailable. Your manuscript and completed work are saved. This review will continue automatically when the service is available.';
    nextAction =
      'No action is needed from you. Please keep this submission; there is no need to upload it again.';
    since = round!.waiting_since!;
    elapsedLabel = 'Elapsed waiting for the review service';
  }
  return {
    waitingForService,
    headline,
    doing,
    nextAction,
    timing: `${elapsedLabel}: ${elapsed(since, now)}.`,
    observation: TIMING_OBSERVATION,
    stage,
    round,
    delivered,
  };
}
