import type { FeedbackLetter, RefereeFile, RefereeRound, Submission } from '../api/types';
import { hasWaitingResponse } from './reviewProgress';

type ReviewStatus = {
  label: string;
  description: string;
  tone: 'good' | 'bad' | 'accent' | 'muted';
};

export function deliveredLetter(round: RefereeRound, letters: readonly FeedbackLetter[]) {
  return letters
    .filter((letter) => letter.round === round.number)
    .sort((a, b) => Date.parse(b.sent_at) - Date.parse(a.sent_at))[0];
}

/** Bind the author's report to the current manuscript, never to an unscoped draft. */
export function authorReview(submission: Submission, file?: RefereeFile) {
  const rounds = [...(file?.rounds ?? [])]
    .filter(
      (round) =>
        round.version === submission.versions.at(-1)?.number &&
        round.claims_revision === submission.claims_revision,
    )
    .sort((a, b) => b.number - a.number);
  const current = rounds[0];
  const settled = ['accepted', 'not_accepted', 'review_complete'].includes(submission.status.state);
  const reportRound = settled
    ? rounds.find((round) => deliveredLetter(round, file?.letters ?? []))
    : undefined;
  const letter = reportRound ? deliveredLetter(reportRound, file?.letters ?? []) : undefined;
  const failed = current
    ? (['referee', 'audit', 'advice', 'letter'] as const).some(
        (key) => (current.progress?.[key] ?? current[key]?.state)?.state === 'failed',
      )
    : false;
  const followup =
    hasWaitingResponse(file, submission) ||
    Boolean(current?.response_ids?.length && current.number !== reportRound?.number);
  let status: ReviewStatus;
  if (submission.status.state === 'withdrawn') {
    status = {
      label: 'Withdrawn',
      description: 'This submission has been withdrawn.',
      tone: 'muted',
    };
  } else if (submission.status.state === 'draft') {
    status = {
      label: 'Draft',
      description: 'Check your compiled PDF, then start the AI review.',
      tone: 'muted',
    };
  } else if (followup) {
    status = {
      label: failed ? 'Follow-up incomplete' : 'Follow-up review',
      description: failed
        ? 'The follow-up review could not be completed. Your response is saved. Any previously delivered report remains available below.'
        : 'Your response is saved and a follow-up review is in progress. Any previously delivered report remains available below.',
      tone: failed ? 'bad' : 'accent',
    };
  } else if (submission.status.state === 'accepted' || submission.status.state === 'not_accepted') {
    const accepted = submission.status.state === 'accepted';
    status = {
      label: accepted ? 'Accepted' : 'Not accepted',
      description: `${accepted ? 'This manuscript was accepted for publication after AI review.' : 'This manuscript was not accepted for publication.'} ${letter ? 'The final report is available below.' : failed ? 'The final report could not be completed; the publication decision is recorded.' : 'The final report is being prepared.'}`,
      tone: accepted ? 'good' : 'bad',
    };
  } else if (letter) {
    status = {
      label: 'Review complete',
      description:
        'Your private AI review is complete. Read the report below for the assessment and suggestions.',
      tone: 'good',
    };
  } else if (failed) {
    status = {
      label: 'Review incomplete',
      description:
        'The review could not be completed. Your manuscript is saved. This status does not identify a mathematical error.',
      tone: 'bad',
    };
  } else {
    status = {
      label: 'Review in progress',
      description:
        submission.status.state === 'review_complete'
          ? 'The assessment is complete and the final report is being prepared. Your private review does not include a publication decision.'
          : 'The AI review is in progress. The final report will appear here when it is ready.',
      tone: 'accent',
    };
  }
  return { current, reportRound, letter, status };
}
