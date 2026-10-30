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
  it('distinguishes a completed analysis awaiting literature from an active review', () => {
    const paper = {
      ...submission,
      decision: {
        decision: 'pending' as const,
        awaiting: 'literature' as const,
        detail: 'Internal comparison C3',
      },
    };
    const view = render(
      <PaperStanding
        submission={paper}
        file={refereeFile({ letters: [feedbackLetter] })}
        onRevise={() => {}}
      />,
    );
    expect(screen.getByRole('heading', { name: 'Awaiting decision' })).toBeVisible();
    expect(screen.getByRole('status')).toHaveTextContent(
      'A comparison with prior work is still needed',
    );
    expect(view.container).not.toHaveTextContent(/under AI|C3|automatically|report is ready/);
    expect(screen.queryByRole('button', { name: 'Upload a revised version' })).toBeNull();
    view.rerender(
      <PaperStanding submission={paper} file={refereeFile({ rounds: [queuedRound] })} />,
    );
    expect(screen.getByRole('heading', { name: 'In review' })).toBeVisible();
    expect(screen.getByRole('status')).toHaveTextContent('Your manuscript is under review.');
    expect(screen.queryByText(/analysis is complete/)).toBeNull();
  });
  it('explains a draft and the confirmation action', () => {
    render(<PaperStanding submission={draft} />);
    expect(screen.getByRole('heading', { name: 'Ready for your check' })).toBeVisible();
    expect(screen.getByText(/Check the manuscript PDF/)).toBeVisible();
    expect(screen.queryByText(/Elapsed since/)).toBeNull();
    expect(screen.queryByText(/Observed on this deployment/)).toBeNull();
  });
  it.each(['accept', 'minor_revision', 'major_revision', 'reject'] as const)(
    'uses the publication decision, independent of the %s recommendation',
    async (verdict) => {
      const revise = vi.fn();
      render(
        <PaperStanding
          submission={accepted}
          file={refereeFile({ rounds: [audited(verdict)], letters: [feedbackLetter] })}
          onRevise={revise}
        />,
      );
      expect(screen.getByRole('heading', { name: 'Accepted' })).toBeVisible();
      expect(screen.getByText('WP-2026-0001')).toBeVisible();
      expect(screen.queryByText(refereeAudit.summary)).toBeNull();
      expect(document.querySelector('.standing-facts')).toBeNull();
      await userEvent.click(screen.getByRole('button', { name: 'Upload a revised version' }));
      expect(revise).toHaveBeenCalledOnce();
    },
  );
  it('shows reasons in sentences and offers a revision', () => {
    render(
      <PaperStanding
        submission={notAccepted}
        file={refereeFile({ rounds: [audited()] })}
        onRevise={() => {}}
      />,
    );
    expect(screen.getByRole('heading', { name: 'Not accepted' })).toBeVisible();
    expect(screen.getByText('Theorem 1.1 already follows from 1901.00001.')).toBeVisible();
    expect(
      screen.getByText(
        'The main results follow from existing results without establishing a distinct new mathematical contribution.',
      ),
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Upload a revised version' })).toBeVisible();
  });
  it.each(['minor_revision', 'major_revision'] as const)(
    'keeps the %s recommendation separate from the publication outcome',
    (verdict) => {
      const round = { ...audited(verdict), number: 2 };
      const { rerender } = render(
        <PaperStanding
          submission={accepted}
          file={refereeFile({ rounds: [round], letters: [feedbackLetter] })}
        />,
      );
      expect(screen.queryByText(/The referee suggests/)).toBeNull();
      rerender(
        <PaperStanding
          submission={accepted}
          file={refereeFile({ rounds: [round], letters: [{ ...feedbackLetter, round: 2 }] })}
        />,
      );
      expect(screen.queryByText(/The referee suggests/)).toBeNull();
      expect(screen.getByRole('heading', { name: 'Accepted' })).toBeVisible();
    },
  );
});

function workspace(person = author, paper = accepted, overrides: Partial<ApiClient> = {}) {
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
    ...renderWithApp(
      <SubmissionDetailPage id={paper.id} staffView={person.roles.includes('editor')} />,
      api,
    ),
    api,
  };
}

describe('standing in the workspace', () => {
  it('places standing, the current letter, readable report, statements and details in order with one referee fetch', async () => {
    const { api, container } = workspace();
    const card = await screen.findByRole('region', { name: 'Paper standing' });
    const report = await screen.findByRole('region', { name: 'Final review report' });
    expect(within(card).queryByText(refereeAudit.summary)).toBeNull();
    expect(within(report).getByRole('heading', { name: 'Review report' })).toBeVisible();
    expect(within(report).queryByText(accepted.title)).toBeNull();
    expect(within(report).getByText(refereeAudit.summary)).toBeVisible();
    expect(card.querySelector('.standing-facts')).toBeNull();
    const results = screen.getByText('Results in this manuscript').closest('details')!;
    expect(results).not.toHaveAttribute('open');
    await userEvent.click(screen.getByText('Results in this manuscript'));
    const statements = screen.getByRole('region', { name: 'Statements' });
    const details = screen.getByText('Manuscript details').closest('details')!;
    for (const [first, second] of [
      [card, report],
      [report, statements],
      [statements, details],
    ]) {
      expect(
        first!.compareDocumentPosition(second!) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }
    expect(details).not.toHaveAttribute('open');
    expect(container.querySelectorAll('.author-details')).toHaveLength(1);
    expect(api.referee).toHaveBeenCalledOnce();
  });
  it.each([accepted, notAccepted])('opens the existing revision upload flow', async (paper) => {
    workspace(author, paper);
    await userEvent.click(await screen.findByRole('button', { name: 'Upload a revised version' }));
    const form = screen.getByRole('form', { name: 'Upload a new version' });
    expect(form.parentElement).toHaveAttribute('id', 'standing-revision-form');
  });
  it('reaches the corrected-source upload directly from a compile-failure standing', async () => {
    workspace(author, {
      ...draft,
      versions: [{ ...draft.versions[0]!, pdf: undefined, compile_error: 'TeX failed' }],
    });
    const card = await screen.findByRole('region', { name: 'Paper standing' });
    expect(within(card).getByRole('heading', { name: 'PDF could not compile' })).toBeVisible();
    await userEvent.click(within(card).getByRole('button', { name: 'Upload a corrected version' }));
    const form = screen.getByRole('form', { name: 'Upload a new version' });
    expect(form.parentElement).toHaveAttribute('id', 'standing-revision-form');
    expect(form.closest('details')).toBeNull();
    expect(within(form).getByLabelText('Revised manuscript files')).toBeVisible();
    expect(within(form).getByLabelText('Revised manuscript files')).toHaveFocus();
    expect(
      screen.getAllByRole('form', { name: 'Upload a new version', hidden: true }),
    ).toHaveLength(1);
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
    await user.upload(within(form).getByLabelText('Revised manuscript files'), source);
    await user.type(within(form).getByLabelText('What changed'), 'Clarified the base case.');
    await user.click(within(form).getByRole('button', { name: 'Upload version' }));
    expect(uploadVersion).toHaveBeenCalledExactlyOnceWith(
      accepted.id,
      source,
      'Clarified the base case.',
      'ai-review-request-2026-10-29',
    );
    expect(await screen.findByRole('form', { name: 'Confirm statements' })).toBeInTheDocument();
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
    workspace(editor);
    const card = await screen.findByRole('region', { name: 'Paper standing' });
    await userEvent.click(screen.getByText('Details: evidence, history and settings'));
    await screen.findByRole('heading', { name: 'Editor tools' });
    expect(within(card).queryByRole('button')).toBeNull();
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
    expect(screen.getByRole('heading', { name: 'In review' })).toBeInTheDocument();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(screen.getAllByRole('heading', { name: 'Accepted' }).length).toBeGreaterThan(0);
    expect(referee).toHaveBeenCalledTimes(3);
    await act(async () => {
      fireEvent.focus(window);
    });
    expect(referee).toHaveBeenCalledTimes(4);
  });
});

it('requires a delivered letter for the current round before showing Review ready', () => {
  const paper = {
    ...submission,
    purpose: 'review_only' as const,
    status: { state: 'review_complete' as const },
  };
  const round = {
    ...audited(),
    number: 2,
    letter: {
      ...refereeRound.letter!,
      state: {
        state: 'done' as const,
        at: refereeRound.started_at,
        result: { subject: 'Hidden draft', body: '', note: '' },
      },
    },
  };
  const { rerender } = render(
    <PaperStanding
      submission={paper}
      file={refereeFile({ rounds: [round], letters: [feedbackLetter] })}
    />,
  );
  expect(screen.queryByRole('heading', { name: 'Review ready' })).toBeNull();
  rerender(
    <PaperStanding
      submission={paper}
      file={refereeFile({ rounds: [round], letters: [{ ...feedbackLetter, round: 2 }] })}
    />,
  );
  expect(screen.getByRole('heading', { name: 'Review ready' })).toBeVisible();
});
