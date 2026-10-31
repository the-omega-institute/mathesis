import { render, screen } from '@testing-library/react';
import { PaperStanding } from '../components/PaperStanding';
import { describe, expect, it } from 'vitest';
import type {
  RefereeFile,
  RefereeRound,
  StageProgress,
  Submission,
  SubmissionStatus,
} from '../api/types';
import { accepted, draft, notAccepted, submission } from '../test/fixtures';
import { feedbackLetter, refereeFile, refereeRound } from '../test/referee';
import { authorStanding, TIMING_OBSERVATION } from './authorStanding';
import { REVIEW_STAGES } from './reviewProgress';
import { SUBMISSION_STATE_LABELS } from './labels';

const at = '2026-10-22T12:00:00Z';
const now = Date.parse('2026-10-22T12:07:00Z');
const privatePaper: Submission = {
  ...submission,
  purpose: 'review_only',
  status: { state: 'review_complete' },
};
function stageRound(
  stage: string,
  state: 'running' | 'pending' | 'failed' = 'running',
): RefereeRound {
  const progress = Object.fromEntries(
    REVIEW_STAGES.map(([key], i) => [
      key,
      i < REVIEW_STAGES.findIndex(([k]) => k === stage)
        ? { state: 'done', at }
        : key === stage
          ? state === 'running'
            ? { state, since: at }
            : state === 'failed'
              ? { state, at, retryable: true }
              : { state }
          : { state: 'pending' },
    ]),
  ) as Record<(typeof REVIEW_STAGES)[number][0], StageProgress>;
  return { ...refereeRound, started_at: at, progress };
}
function file(round: RefereeRound, sent = false) {
  return refereeFile({
    rounds: [round],
    letters: sent ? [{ ...feedbackLetter, round: round.number, sent_at: at }] : [],
  });
}
type StandingCase = [string, Submission, RefereeFile | undefined, string, string];
// Every submission state must explain itself even when no review file is available.
const noRoundCases = {
  draft: [
    'statement confirmation',
    {
      ...draft,
      versions: [
        {
          ...draft.versions[0]!,
          uploaded_at: at,
          pdf: { id: 'pdf', bytes: 1, sha256: 'a'.repeat(64) },
        },
      ],
    },
    undefined,
    'Ready for your check',
    'Check the manuscript PDF',
  ],
  in_review: [
    'review progress unavailable without a file',
    submission,
    undefined,
    'In review',
    'Nothing to do now',
  ],
  review_complete: [
    'completed private review without a current round',
    privatePaper,
    undefined,
    'Review progress unavailable',
    'Read the available report',
  ],
  accepted: [
    'accepted without a current round',
    accepted,
    undefined,
    'Accepted',
    'Nothing is required',
  ],
  not_accepted: [
    'not accepted without a current round',
    notAccepted,
    undefined,
    'Not accepted',
    'upload a corrected version',
  ],
  withdrawn: [
    'withdrawn',
    { ...submission, status: { state: 'withdrawn' } },
    undefined,
    'Withdrawn',
    'Nothing to do now',
  ],
} satisfies Record<SubmissionStatus['state'], StandingCase>;
const cases: StandingCase[] = [
  ...Object.values(noRoundCases),
  [
    'PDF compilation',
    {
      ...draft,
      versions: [
        { ...draft.versions[0]!, uploaded_at: at, pdf: undefined, compile_error: undefined },
      ],
    },
    undefined,
    'PDF compiling',
    'Check the manuscript PDF when',
  ],
  [
    'compile failure',
    {
      ...draft,
      versions: [
        { ...draft.versions[0]!, uploaded_at: at, pdf: undefined, compile_error: 'TeX failed' },
      ],
    },
    undefined,
    'PDF could not compile',
    'corrected version',
  ],
  ...REVIEW_STAGES.map(([stage, label]): (typeof cases)[number] => [
    label,
    stage === 'formal' ? accepted : submission,
    file(stageRound(stage), stage === 'formal'),
    stage === 'formal' ? 'Accepted' : 'In review',
    stage === 'formal' ? 'Read the analysis and suggestions' : 'Nothing to do now',
  ]),
  [
    'private letter delivered',
    privatePaper,
    file({ ...refereeRound, formal: undefined }, true),
    'Review ready',
    'Read the analysis and suggestions',
  ],
  ['accepted', accepted, file(refereeRound, true), 'Accepted', 'Read the analysis and suggestions'],
  [
    'not accepted',
    notAccepted,
    file(refereeRound, true),
    'Not accepted',
    'Read the analysis and suggestions',
  ],
  [
    'private review before letter',
    privatePaper,
    file(stageRound('letter')),
    'Private review in progress',
    'Nothing to do now',
  ],
];

describe('one author standing for every state', () => {
  it('covers the complete submission status vocabulary in the state table', () => {
    expect(Object.keys(noRoundCases).sort()).toEqual(Object.keys(SUBMISSION_STATE_LABELS).sort());
  });
  it.each([
    '22 Oct 2026 for a four-page note',
    'PDF under 1 min;',
    'Referee report 19 min;',
    'Source check 8 min;',
    'Improvement suggestions 9 min;',
    'Feedback letter 2 min;',
    'about 45 min from upload to letter.',
    'Totals vary with the queue and retries;',
    'this is not a promise or deadline.',
  ])('pins the measured manuscript observation: %s', (observation) => {
    expect(TIMING_OBSERVATION).toContain(observation);
  });
  it.each([
    undefined,
    refereeFile({ rounds: [] }),
    file({ ...refereeRound, version: 0 }),
    file({ ...refereeRound, claims_revision: -1 }),
  ])('does not infer a queue when current progress is unavailable', (review) => {
    const standing = authorStanding(submission, review, now);
    expect(standing.doing).toBe(
      'Progress details are loading or unavailable; the review is in progress on the server.',
    );
    expect(standing.nextAction).toContain('Nothing to do now');
    expect(standing.stage).toBeUndefined();
  });
  it('only describes a queued Referee report when its stage is pending', () => {
    const standing = authorStanding(submission, file(stageRound('referee', 'pending')), now);
    expect(standing.doing).toBe('Your review is queued and will continue automatically.');
    expect(standing.stage).toBe('referee');
    expect(standing.headline).toBe(SUBMISSION_STATE_LABELS.in_review);
  });
  it.each(cases)(
    '%s explains doing, next action and elapsed time with dated observations',
    (_name, paper, review, headline, next) => {
      const standing = authorStanding(paper, review, now);
      expect(standing.headline).toBe(headline);
      expect(standing.doing).not.toBe('');
      expect(standing.nextAction).toContain(next);
      expect(standing.timing).toMatch(/^Elapsed .+: (less than 1 min|\d+ min|\d+ h \d+ min)\./);
      expect(standing.observation).toBe(TIMING_OBSERVATION);
      expect(standing.observation).toContain('22 Oct 2026');
      expect(standing.observation).toContain('not a promise or deadline');
      expect(JSON.stringify(standing)).not.toMatch(/try again/i);
      render(<PaperStanding submission={paper} file={review} />);
      expect(screen.getByRole('heading', { name: headline })).toBeVisible();
      expect(screen.getByRole('status').textContent).not.toBe('');
      expect(screen.queryByText(/^Elapsed /)).toBeNull();
      expect(screen.getByRole('status')).not.toHaveTextContent(
        /Source check|Feedback letter|server|execution/i,
      );
      expect(screen.queryByText(TIMING_OBSERVATION)).toBeNull();
    },
  );
  it.each(REVIEW_STAGES)('%s uses safe failure text', (stage, label) => {
    const standing = authorStanding(
      stage === 'formal' ? accepted : submission,
      file(stageRound(stage, 'failed'), stage === 'formal'),
      now,
    );
    expect(standing.doing).toContain(`${label} needs attention`);
    expect(standing.doing).toContain('upload and any report are saved');
    expect(standing.doing).toContain('not a verdict on the mathematics');
    expect(standing.nextAction).toContain('recover');
    expect(JSON.stringify(standing)).not.toMatch(/try again/i);
  });
  it.each(REVIEW_STAGES)(
    '%s uses its actual running timestamp and queue timestamp',
    (stage, label) => {
      const paper = stage === 'formal' ? accepted : submission;
      expect(authorStanding(paper, file(stageRound(stage), stage === 'formal'), now).timing).toBe(
        `Elapsed since ${label} started: 7 min.`,
      );
      expect(
        authorStanding(paper, file(stageRound(stage, 'pending'), stage === 'formal'), now).timing,
      ).toBe(`Elapsed waiting for ${label}: 7 min.`);
    },
  );
  it('keeps a saved response pending until admitted, then waits for its own letter', () => {
    const response = {
      id: 'response',
      round: 1,
      concern: 0,
      version: submission.versions.at(-1)!.number,
      claims_revision: submission.claims_revision,
      body: 'Checked.',
      author: submission.submitter,
      at,
    };
    const saved = refereeFile({
      rounds: [refereeRound],
      letters: [feedbackLetter],
      finding_responses: [response],
    });
    const waiting = authorStanding(privatePaper, saved, now);
    expect(waiting.headline).toBe('Your response is awaiting review');
    expect(waiting.nextAction).toContain('Nothing to do now');
    expect(waiting.timing).toBe('Elapsed since your response was saved: 7 min.');
    expect(authorStanding(privatePaper, { ...saved, rounds: [] }, now).doing).toBe(waiting.doing);
    expect(
      authorStanding(accepted, { ...saved, rounds: [stageRound('formal')] }, now).stage,
    ).toBeUndefined();
    const admitted = {
      ...saved,
      rounds: [...saved.rounds, { ...stageRound('audit'), number: 2, response_ids: ['response'] }],
    };
    expect(authorStanding(privatePaper, admitted, now).headline).toBe(
      'Your response is being reviewed',
    );
    expect(authorStanding(privatePaper, admitted, now).delivered).toBe(false);
  });
  it.each([privatePaper, accepted, notAccepted])(
    '$status.state does not invent queued work without a current round',
    (paper) => {
      for (const review of [
        undefined,
        refereeFile({ rounds: [], letters: [feedbackLetter] }),
        file({ ...refereeRound, version: 0 }, true),
        file({ ...refereeRound, claims_revision: -1 }, true),
      ]) {
        const standing = authorStanding(paper, review, now);
        expect(standing.round).toBeUndefined();
        expect(standing.delivered).toBe(false);
        expect(standing.stage).toBeUndefined();
        expect(standing.headline).not.toBe('Review ready');
        expect(standing.doing).toContain('Progress details for this version are unavailable');
        expect(standing.doing).toContain('Feedback letters, reports and review history');
        expect(standing.doing).not.toMatch(/waiting|queued|in progress/i);
        expect(standing.nextAction).toContain(noRoundCases[paper.status.state][4]);
      }
    },
  );
});

it('shows recorded service delays without exposing execution details or offering revision', () => {
  const delayed = { ...stageRound('audit'), waiting_since: at };
  const report = file(delayed);
  const standing = authorStanding(submission, report, now);
  expect(standing.headline).toBe('Review temporarily delayed');
  render(<PaperStanding submission={submission} file={report} onRevise={() => {}} />);
  expect(screen.getByRole('status')).toHaveTextContent('completed work are saved');
  expect(screen.getByRole('status')).toHaveTextContent('continue automatically');
  expect(screen.queryByRole('button', { name: /revis/i })).not.toBeInTheDocument();
  expect(screen.getByRole('status')).not.toHaveTextContent(/CMA|503|Codex|audit/i);
  expect(authorStanding(submission, file(stageRound('audit')), now).waitingForService).toBe(false);
  expect(authorStanding(accepted, report, now).headline).toBe('Accepted');
  expect(authorStanding(privatePaper, file(delayed, true), now).waitingForService).toBe(false);
});
