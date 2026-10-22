import type { RefereeFile, Submission, SubmissionStatus } from '../api/types';
import { accepted, draft, notAccepted, submission } from './fixtures';
import { feedbackLetter, queuedRound, refereeFile } from './referee';

const delivered = refereeFile({ letters: [feedbackLetter] });
const privateReview: Submission = {
  ...submission,
  purpose: 'review_only',
  status: { state: 'review_complete' },
};

/** The author surfaces must use the same headline across every submission state. */
type Case = { paper: Submission; review: RefereeFile };
const withDelivered = (paper: Submission): Case => ({ paper, review: delivered });
const failedReferee: Case = {
  paper: submission,
  review: refereeFile({
    rounds: [
      {
        ...queuedRound,
        referee: {
          ...queuedRound.referee,
          state: {
            state: 'failed',
            reason: 'Review failed',
            retryable: true,
            at: queuedRound.started_at,
          },
        },
      },
    ],
  }),
};
/** Keyed by the status union so a new state cannot be left out of the table. */
export const authorStandingCasesByState = {
  draft: [
    withDelivered(draft),
    withDelivered({
      ...draft,
      versions: [{ ...draft.versions[0]!, pdf: undefined, compile_error: 'TeX failed' }],
    }),
  ],
  in_review: [withDelivered(submission), failedReferee],
  review_complete: [
    withDelivered(privateReview),
    { paper: privateReview, review: refereeFile({ rounds: [] }) },
    { paper: privateReview, review: refereeFile({ rounds: [queuedRound] }) },
  ],
  accepted: [withDelivered(accepted), withDelivered({ ...accepted, kind: 'conjecture' as const })],
  not_accepted: [
    withDelivered(notAccepted),
    withDelivered({ ...notAccepted, kind: 'conjecture' as const }),
  ],
  withdrawn: [withDelivered({ ...submission, status: { state: 'withdrawn' as const } })],
} satisfies Record<SubmissionStatus['state'], Case[]>;
export const authorStandingCases: Case[] = Object.values(authorStandingCasesByState).flat();
