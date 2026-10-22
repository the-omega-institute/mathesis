import type { RefereeFile, Submission } from '../api/types';
import { accepted, draft, notAccepted, submission } from './fixtures';
import { feedbackLetter, queuedRound, refereeFile } from './referee';

const delivered = refereeFile({ letters: [feedbackLetter] });
const privateReview: Submission = {
  ...submission,
  purpose: 'review_only',
  status: { state: 'review_complete' },
};

/** The author surfaces must use the same headline across every submission state. */
export const authorStandingCases = [
  ...[
    draft,
    submission,
    privateReview,
    accepted,
    notAccepted,
    { ...submission, status: { state: 'withdrawn' as const } },
    {
      ...draft,
      versions: [{ ...draft.versions[0]!, pdf: undefined, compile_error: 'TeX failed' }],
    },
    { ...accepted, kind: 'conjecture' as const },
    { ...notAccepted, kind: 'conjecture' as const },
  ].map((paper) => ({ paper, review: delivered })),
  {
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
  },
  { paper: privateReview, review: refereeFile({ rounds: [] }) },
  { paper: privateReview, review: refereeFile({ rounds: [queuedRound] }) },
] satisfies { paper: Submission; review: RefereeFile }[];
