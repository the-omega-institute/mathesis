import { act, fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ApiClient } from '../api/client';
import type { RefereeRound } from '../api/types';
import { SubmissionDetailPage } from '../pages/SubmissionDetail';
import {
  accepted,
  analysis,
  author,
  draft,
  editor,
  notAccepted,
  submission,
} from '../test/fixtures';
import {
  feedbackLetter,
  queuedRound,
  refereeAudit,
  refereeFile,
  refereeRound,
} from '../test/referee';
import { fakeApi, renderWithApp } from '../test/render';
import { PaperStanding } from './PaperStanding';

afterEach(() => vi.useRealTimers());

function audited(verdict = refereeAudit.verdict): RefereeRound {
  return {
    ...refereeRound,
    audit: {
      attempts: 1,
      state: { state: 'done', at: '2026-10-09T00:00:00Z', result: { ...refereeAudit, verdict } },
    },
  };
}

describe('paper standing', () => {
  it('renders nothing for a draft', () => {
    const { container } = render(<PaperStanding submission={draft} round={audited()} />);
    expect(container).toBeEmptyDOMElement();
  });
  it.each([
    ['Referee', queuedRound, 'The referee is reading your paper.'],
    [
      'Check',
      { ...queuedRound, referee: refereeRound.referee },
      'The referee’s report is being checked against your source.',
    ],
    ['Letter', { ...audited(), letter: queuedRound.letter }, 'The letter is being written.'],
  ] as const)('shows progress at the %s stage', (current, round, phrase) => {
    render(
      <PaperStanding
        submission={submission}
        round={round}
        letters={[{ ...feedbackLetter, assessment: 'reject' }]}
      />,
    );
    const card = screen.getByRole('region', { name: 'Paper standing' });
    expect(within(card).getByRole('heading', { name: 'Under review', level: 2 })).toBeVisible();
    const steps = within(within(card).getByRole('list', { name: 'Review progress' })).getAllByRole(
      'listitem',
    );
    expect(steps.map((step) => step.textContent)).toEqual([
      'Referee',
      'Check',
      'Decision',
      'Letter',
    ]);
    expect(steps.filter((step) => step.getAttribute('aria-current') === 'step')).toEqual([
      within(card).getByText(current, { selector: 'li' }),
    ]);
    expect(within(card).getByRole('status')).toHaveTextContent(phrase);
    expect(within(card).queryByText(refereeAudit.summary)).toBeNull();
    expect(within(card).queryByText('statements')).toBeNull();
  });
  it.each(['accept', 'minor_revision', 'major_revision', 'reject'] as const)(
    'uses acceptance rather than the %s recommendation',
    async (verdict) => {
      const revise = vi.fn();
      render(<PaperStanding submission={accepted} round={audited(verdict)} onRevise={revise} />);
      expect(screen.getByRole('heading', { name: 'Accepted' })).toBeInTheDocument();
      expect(screen.getByText('WP-2026-0001')).toBeVisible();
      expect(screen.getByText(refereeAudit.summary)).toBeInTheDocument();
      expect(screen.getAllByRole('listitem').map((fact) => fact.textContent)).toEqual([
        '2 statements',
        '2 correct',
        '1 proved in Lean',
      ]);
      expect(screen.queryByRole('list', { name: 'Review progress' })).toBeNull();
      expect(screen.queryByRole('status')).toBeNull();
      if (verdict === 'minor_revision' || verdict === 'major_revision') {
        expect(
          screen.getByText(
            `The referee suggests ${verdict === 'minor_revision' ? 'minor' : 'major'} revisions — see the letter.`,
          ),
        ).toBeInTheDocument();
        await userEvent.click(screen.getByRole('button', { name: 'Upload a revised version' }));
        expect(revise).toHaveBeenCalledOnce();
      } else expect(screen.queryByRole('button')).toBeNull();
    },
  );
  it('shows published reasons in plain words and permits revision despite an accept verdict', async () => {
    const revise = vi.fn();
    render(<PaperStanding submission={notAccepted} round={audited('accept')} onRevise={revise} />);
    expect(screen.getByRole('heading', { name: 'Not accepted' })).toBeInTheDocument();
    expect(screen.getByText('Statement C1 already follows from 1901.00001.')).toBeInTheDocument();
    expect(
      screen.getByText('No checked main result carries new mathematical content.'),
    ).toBeInTheDocument();
    expect(screen.getByText(refereeAudit.summary)).toBeInTheDocument();
    expect(screen.getByText('You can revise the paper and submit a new version.')).toBeVisible();
    expect(screen.queryByText('WP-2026-0001')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Upload a revised version' }));
    expect(revise).toHaveBeenCalledOnce();
  });
  it('handles a legacy paper without an audit and a withdrawn paper', () => {
    const { rerender } = render(<PaperStanding submission={accepted} />);
    expect(screen.getByRole('heading', { name: 'Accepted' })).toBeInTheDocument();
    expect(screen.queryByRole('list')).toBeNull();
    rerender(<PaperStanding submission={{ ...submission, status: { state: 'withdrawn' } }} />);
    expect(screen.getByRole('heading', { name: 'Withdrawn' })).toBeInTheDocument();
  });
  it('omits Lean facts when there is no formal probe', () => {
    render(<PaperStanding submission={accepted} round={{ ...audited(), formal: undefined }} />);
    expect(screen.getAllByRole('listitem').map((fact) => fact.textContent)).toEqual([
      '2 statements',
      '2 correct',
    ]);
    expect(screen.queryByText(/proved in Lean/)).toBeNull();
  });
});

function workspace(
  person = author,
  paper = accepted,
  overrides: Partial<ApiClient> = {},
  staffOnly = false,
) {
  const api = fakeApi(
    {
      getSubmission: vi.fn(async () => paper),
      previewDecision: vi.fn(async () => paper.decision!),
      getAnalysis: vi.fn(async () => analysis),
      referee: vi.fn(async () =>
        refereeFile({ letters: [{ ...feedbackLetter, assessment: 'minor_revision' }] }),
      ),
      ...overrides,
    },
    person,
  );
  return {
    ...renderWithApp(<SubmissionDetailPage id={paper.id} staffOnly={staffOnly} />, api),
    api,
  };
}

describe('standing in the workspace', () => {
  it('shows the final report within the review tab with one referee fetch', async () => {
    const { api } = workspace();
    const report = await screen.findByRole('region', { name: 'AI review report' });
    expect(report.closest('[role=tabpanel]')).toHaveAccessibleName('Review');
    expect(within(report).getByRole('heading', { name: 'Review report' })).toBeVisible();
    expect(within(report).getByText('Accepted', { selector: '.badge' })).toBeVisible();
    expect(within(report).getByRole('heading', { name: 'Accepted statements' })).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Paper standing' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Statements' })).toBeNull();
    expect(screen.queryByText('Source audit')).toBeNull();
    expect(api.referee).toHaveBeenCalledOnce();
  });
  it.each([accepted, notAccepted])('opens the existing revision upload flow', async (paper) => {
    workspace(author, paper);
    await userEvent.click(await screen.findByRole('button', { name: 'Upload a revised version' }));
    const form = screen.getByRole('form', { name: 'Upload a new version' });
    expect(form.parentElement).toHaveAttribute('id', 'standing-revision-form');
  });
  it('uploads a revision and returns to statement confirmation', async () => {
    let current = accepted;
    const uploadVersion = vi.fn(async () => {
      current = draft;
      return current;
    });
    workspace(author, accepted, { uploadVersion, getSubmission: vi.fn(async () => current) });
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: 'Upload a revised version' }));
    const form = screen.getByRole('form', { name: 'Upload a new version' });
    const source = new File(['source'], 'revision.tex', { type: 'text/plain' });
    await user.upload(within(form).getByLabelText('Revised LaTeX source'), source);
    await user.type(within(form).getByLabelText('What changed'), 'Clarified the base case.');
    await user.click(within(form).getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(within(form).getByRole('button', { name: 'Upload version' }));
    expect(uploadVersion).toHaveBeenCalledExactlyOnceWith(
      accepted.id,
      source,
      'Clarified the base case.',
      'ai-review-2026-10-11',
    );
    expect(await screen.findByRole('form', { name: 'Start review' })).toBeInTheDocument();
  });
  it('offers a linked co-author revision, while staff keep their tools', async () => {
    const { unmount } = workspace(author, {
      ...accepted,
      submitter: 'someone-else',
      authors: [{ name: 'Coauthor', person: author.id }],
    });
    expect(
      await screen.findByRole('button', { name: 'Upload a revised version' }),
    ).toBeInTheDocument();
    unmount();
    workspace(editor, accepted, {}, true);
    const report = await screen.findByRole('region', { name: 'Review overview' });
    expect(within(report).getByRole('heading', { name: 'What is complete' })).toBeInTheDocument();
    expect(within(report).getByText('Accepted', { selector: '.badge' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('tab', { name: 'Advanced' }));
    expect(screen.getByRole('heading', { name: 'Editor tools' })).toBeInTheDocument();
  });
  it('refreshes the applied paper decision with referee polling', async () => {
    vi.useFakeTimers();
    const referee = vi
      .fn<ApiClient['referee']>()
      .mockResolvedValueOnce(refereeFile({ revision: 1, rounds: [queuedRound], letters: [] }))
      .mockResolvedValue(refereeFile({ revision: 2, rounds: [audited()] }));
    const getSubmission = vi
      .fn<ApiClient['getSubmission']>()
      .mockResolvedValueOnce(submission)
      .mockResolvedValueOnce(submission)
      .mockResolvedValue({ ...accepted, revision: submission.revision + 1 });
    workspace(author, submission, { referee, getSubmission });
    await act(async () => {});
    expect(screen.getByRole('heading', { name: 'Review report' })).toBeInTheDocument();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(screen.getAllByText('Accepted', { selector: '.badge' }).length).toBeGreaterThan(0);
    expect(referee).toHaveBeenCalledTimes(3);
    await act(async () => {
      fireEvent.focus(window);
    });
    expect(referee).toHaveBeenCalledTimes(4);
  });
});

it('does not infer active letter writing from the author projection', () => {
  render(<PaperStanding submission={submission} round={{ ...audited(), letter: undefined }} />);
  expect(
    screen.getByText(
      'The source audit is complete. The feedback letter has not been delivered yet.',
    ),
  ).toBeInTheDocument();
  expect(screen.queryByText('The letter is being written.')).toBeNull();
});

it('keeps a private re-review pending until its own letter is delivered', () => {
  const paper = {
    ...submission,
    purpose: 'review_only' as const,
    status: { state: 'review_complete' as const },
  };
  const followup = { ...queuedRound, number: 2, response_ids: ['response-1'] };
  const { rerender } = render(
    <PaperStanding submission={paper} round={followup} letters={[feedbackLetter]} />,
  );
  expect(screen.getByRole('heading', { name: 'Your response is being reviewed' })).toBeVisible();
  expect(screen.getByRole('list', { name: 'Review progress' })).toBeVisible();
  expect(screen.queryByText('Your private review is ready')).toBeNull();
  const checked = { ...audited(), number: 2, letter: undefined };
  rerender(<PaperStanding submission={paper} round={checked} letters={[feedbackLetter]} />);
  expect(screen.getByRole('heading', { name: 'Your source audit is ready' })).toBeVisible();
  expect(screen.getByRole('status')).toHaveTextContent(
    'The feedback letter has not been delivered yet.',
  );
  rerender(
    <PaperStanding
      submission={paper}
      round={checked}
      letters={[feedbackLetter, { ...feedbackLetter, round: 2 }]}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Your private review is ready' })).toBeVisible();
  expect(screen.queryByRole('list', { name: 'Review progress' })).toBeNull();
});

it('keeps a completed but undelivered letter pending and shows post-decision failures', () => {
  const at = refereeRound.started_at;
  const progress = {
    referee: { state: 'done' as const, at },
    audit: { state: 'done' as const, at },
    advice: { state: 'done' as const, at },
    letter: { state: 'done' as const, at },
    formal: { state: 'skipped' as const },
  };
  const paper = {
    ...submission,
    purpose: 'review_only' as const,
    status: { state: 'review_complete' as const },
  };
  const round = { ...audited(), progress, letter: undefined, number: 2 };
  const { rerender } = render(
    <PaperStanding submission={paper} round={round} letters={[feedbackLetter]} />,
  );
  expect(screen.queryByText('Your private review is ready')).toBeNull();
  expect(screen.getByRole('status')).toHaveTextContent(
    'No feedback letter has been delivered for this round.',
  );
  rerender(
    <PaperStanding
      submission={accepted}
      round={{
        ...round,
        progress: { ...progress, letter: { state: 'failed', at, retryable: true } },
      }}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Accepted' })).toBeVisible();
  expect(screen.getByRole('status')).toHaveTextContent('Feedback letter needs attention.');
});
