import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ApiError } from '../api/client';
import {
  accepted,
  analysis,
  author,
  draft,
  editor,
  extracted,
  notAccepted,
  submission,
  version,
} from '../test/fixtures';
import { KATEX_ERROR, MACROS } from '../test/katex';
import { fakeApi, renderWithApp } from '../test/render';
import { SubmissionDetailPage } from './SubmissionDetail';

describe('paper workspace', () => {
  it('guards the staff manuscript route before fetching a private submission', async () => {
    const getSubmission = vi.fn(async () => accepted);
    renderWithApp(
      <SubmissionDetailPage id={accepted.id} staffOnly />,
      fakeApi({ getSubmission }, author),
    );
    expect(await screen.findByRole('heading', { name: 'Staff access required' })).toBeVisible();
    expect(getSubmission).not.toHaveBeenCalled();
  });

  it('refreshes a delayed conjecture target on focus after the review has settled', async () => {
    const initial: typeof accepted = {
      ...accepted,
      kind: 'conjecture' as const,
      claims: [{ ...accepted.claims[2]!, role: 'main' as const }],
      lean_statements: [],
    };
    let current = initial;
    const getSubmission = vi.fn(async () => current);
    renderWithApp(
      <SubmissionDetailPage id={initial.id} />,
      fakeApi(
        {
          getSubmission,
          getAnalysis: vi.fn(async () => analysis),
          previewDecision: vi.fn(async () => initial.decision!),
        },
        author,
      ),
    );
    await userEvent.click(await screen.findByRole('tab', { name: 'Formalization' }));
    expect(await screen.findByText('The Lean statement is being prepared.')).toBeInTheDocument();
    current = {
      ...initial,
      revision: initial.revision + 1,
      lean_statements: [
        {
          claim: 'C3',
          version: 1,
          claims_revision: initial.claims_revision,
          digest: 'a'.repeat(64),
          lean: 'theorem wishpool_target : True := by sorry',
          toolchain: 'Lean test',
          reading: 'A delayed translation.',
          faithfulness: {
            verdict: 'faithful',
            reason: 'Quantifiers match.',
            mismatch: '',
            regenerated: false,
          },
          response: { state: 'awaiting_author' as const },
          created_at: '2026-10-10T00:00:00Z',
        },
      ],
    };
    fireEvent.focus(window);
    expect(await screen.findByText('A delayed translation.')).toBeInTheDocument();
    expect(
      await screen.findByRole('button', { name: 'Yes, this matches my statement' }),
    ).toBeEnabled();
  });

  it('lets the author confirm the extracted statements', async () => {
    const confirmClaims = vi.fn(async () => submission);
    const api = fakeApi(
      {
        getSubmission: vi.fn(async () => draft),
        previewDecision: vi.fn(async () => draft.decision!),
        confirmClaims,
      },
      author,
    );
    renderWithApp(<SubmissionDetailPage id="sub-draft" />, api);
    const user = userEvent.setup();

    const form = await screen.findByRole('form', { name: 'Start review' });
    expect(screen.queryByRole('heading', { name: 'Orchestra review' })).toBeNull();
    expect(screen.getByRole('heading', { name: 'Check the compiled PDF' })).toBeInTheDocument();
    expect(screen.getByText(/has no number; using "Conjecture"/)).toBeInTheDocument();
    // The statement is rendered with KaTeX.
    expect(form.querySelector('.katex')).not.toBeNull();

    await user.click(within(form).getByText(/statements C1 uses/));
    await user.click(await within(form).findByRole('checkbox', { name: 'C1 uses C2' }));
    await user.selectOptions(within(form).getByLabelText('Kind of C2'), 'proposition');
    await user.selectOptions(within(form).getByLabelText('Role of C3'), 'main');
    await user.click(within(form).getByRole('checkbox', { name: 'Exclude C4' }));
    await user.click(screen.getByRole('button', { name: 'Start review with these statements' }));

    expect(confirmClaims).toHaveBeenCalledWith('sub-draft', [
      { id: 'C1', kind: 'theorem', role: 'main', depends_on: ['C2'] },
      { id: 'C2', kind: 'proposition', role: 'supporting', depends_on: [] },
      { id: 'C3', kind: 'conjecture', role: 'main', depends_on: [] },
      { id: 'C4', kind: 'claim', role: 'supporting', excluded: true },
    ]);
    await waitFor(() => expect(screen.queryByRole('form', { name: 'Start review' })).toBeNull());
  });

  it('renders the confirm table with the current version’s macros and full select labels', async () => {
    const withMacros = {
      ...draft,
      // Only the current (last) version's macros apply.
      versions: [version(1), version(2, { macros: MACROS })],
      extracted: [
        { ...extracted[0]!, statement: 'The map $\\rep$ satisfies $\\code{ab}$.' },
        ...extracted.slice(1),
      ],
    };
    const api = fakeApi(
      {
        getSubmission: vi.fn(async () => withMacros),
        previewDecision: vi.fn(async () => draft.decision!),
      },
      author,
    );
    renderWithApp(<SubmissionDetailPage id="sub-draft" />, api);
    const form = await screen.findByRole('form', { name: 'Start review' });
    expect(form.querySelector('.mop')).toHaveTextContent('rep');
    expect(form.querySelector(KATEX_ERROR)).toBeNull();

    // Kind and Role sit in their own fitted columns and show their full option text.
    const kind = within(form).getByLabelText('Kind of C2');
    const role = within(form).getByLabelText('Role of C2');
    expect(kind).toHaveDisplayValue('Lemma');
    expect(role).toHaveDisplayValue('Supporting');
    expect(within(form).getByLabelText('Role of C1')).toHaveDisplayValue('Main result');
    expect(
      within(kind as HTMLSelectElement)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['Theorem', 'Proposition', 'Lemma', 'Corollary', 'Claim', 'Conjecture', 'Question']);
    expect(kind.closest('td')).toHaveClass('confirm-kind');
    expect(kind.closest('td')).toHaveAttribute('data-label', 'Kind');
    expect(role.closest('td')).toHaveClass('confirm-role');
    expect(role.closest('td')).toHaveAttribute('data-label', 'Role');
    expect(within(form).getByRole('columnheader', { name: 'Kind' })).toHaveClass('confirm-kind');
  });

  it('shows the server’s reason when the confirmation is refused', async () => {
    const api = fakeApi(
      {
        getSubmission: vi.fn(async () => draft),
        previewDecision: vi.fn(async () => draft.decision!),
        confirmClaims: vi.fn(async () => {
          throw new ApiError({
            status: 422,
            code: 'invalid',
            detail: 'claim dependencies contain a cycle',
          });
        }),
      },
      author,
    );
    renderWithApp(<SubmissionDetailPage id="sub-draft" />, api);
    await userEvent.click(
      await screen.findByRole('button', { name: 'Start review with these statements' }),
    );
    expect(await screen.findByText('claim dependencies contain a cycle')).toBeInTheDocument();
  });

  it('gives a paper that was not accepted its reasons and a new-version form', async () => {
    const api = fakeApi(
      {
        getSubmission: vi.fn(async () => notAccepted),
        previewDecision: vi.fn(async () => notAccepted.decision!),
        getAnalysis: vi.fn(async () => ({ ...analysis, submission: 'sub-na' })),
      },
      author,
    );
    renderWithApp(<SubmissionDetailPage id="sub-na" />, api);

    const section = await screen.findByRole('region', { name: 'AI review report' });
    expect(
      within(section).getByText('Statement C1 already follows from 1901.00001.'),
    ).toBeInTheDocument();
    expect(
      within(section).getByText('No checked main result carries new mathematical content.'),
    ).toBeInTheDocument();
    await userEvent.click(
      within(section).getByRole('button', { name: 'Upload a revised version' }),
    );
    expect(screen.getByRole('form', { name: 'Upload a new version' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Editor tools' })).toBeNull();
  });

  it('lets the author of an accepted paper choose visibility and answer a formalization proposal', async () => {
    const setVisibility = vi.fn(async () => ({
      ...accepted,
      analysis_visibility: 'public' as const,
    }));
    const respondFormalization = vi.fn(async () => accepted);
    const api = fakeApi(
      {
        getSubmission: vi.fn(async () => accepted),
        previewDecision: vi.fn(async () => accepted.decision!),
        getAnalysis: vi.fn(async () => analysis),
        setVisibility,
        respondFormalization,
      },
      author,
    );
    renderWithApp(<SubmissionDetailPage id="sub-acc" />, api);
    const user = userEvent.setup();

    expect(await screen.findByRole('link', { name: 'Public page WP-2026-0001' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001',
    );
    await user.click(screen.getByRole('tab', { name: 'Manuscript' }));
    await user.click(screen.getByRole('radio', { name: /^Public/ }));
    expect(setVisibility).toHaveBeenCalledWith('sub-acc', 'public');

    await user.click(screen.getByRole('tab', { name: 'Formalization' }));
    await user.click(screen.getAllByRole('button', { name: 'Decline' })[0]!);
    expect(await screen.findByText('Give a reason for declining.')).toBeInTheDocument();
    await user.type(
      screen.getAllByLabelText('Reason (needed when declining)')[0]!,
      'Not before v2.',
    );
    await user.click(screen.getAllByRole('button', { name: 'Decline' })[0]!);
    expect(respondFormalization).toHaveBeenCalledWith('sub-acc', 'C1', {
      approve: false,
      reason: 'Not before v2.',
    });
  });

  it('shows editors their tools and the 409 detail when deciding too early', async () => {
    const api = fakeApi(
      {
        getSubmission: vi.fn(async () => submission),
        previewDecision: vi.fn(async () => submission.decision!),
        getAnalysis: vi.fn(async () => ({ ...analysis, submission: 'sub-1' })),
        applyDecision: vi.fn(async () => {
          throw new ApiError({
            status: 409,
            code: 'conflict',
            detail: 'awaiting S2: S2 needs a human report',
          });
        }),
      },
      editor,
    );
    renderWithApp(<SubmissionDetailPage id="sub-1" staffOnly />, api);
    await userEvent.click(await screen.findByText('Publication decision', { selector: 'h2' }));
    expect(screen.getByText('Pending — awaiting S2 Literature')).toBeVisible();
    await userEvent.click(screen.getByRole('tab', { name: 'Advanced' }));
    expect(screen.getByRole('heading', { name: 'Editor tools' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Apply decision' }));
    expect(await screen.findByText('awaiting S2: S2 needs a human report')).toBeInTheDocument();
  });
});

it('keeps an admin who authored the paper in the author workspace until they use the staff route', async () => {
  const person = { ...author, roles: ['admin' as const] };
  const api = fakeApi({ getSubmission: vi.fn(async () => accepted) }, person);
  const { unmount } = renderWithApp(<SubmissionDetailPage id={accepted.id} />, api);
  await screen.findByRole('region', { name: 'AI review report' });
  expect(screen.getAllByRole('tab').map((tab) => tab.textContent)).toEqual([
    'Manuscript',
    'Review',
    'Formalization',
  ]);
  expect(screen.queryByRole('region', { name: 'Review overview' })).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Editor tools' })).toBeNull();
  expect(api.getAnalysis).not.toHaveBeenCalled();
  expect(api.previewDecision).not.toHaveBeenCalled();
  expect(screen.getByRole('link', { name: 'Open staff workspace' })).toHaveAttribute(
    'href',
    `/staff/submissions/${accepted.id}`,
  );
  unmount();
  renderWithApp(<SubmissionDetailPage id={accepted.id} staffOnly />, api);
  expect(await screen.findByRole('region', { name: 'Review overview' })).toBeVisible();
  expect(screen.queryByRole('region', { name: 'AI review report' })).toBeNull();
  expect(screen.getByRole('link', { name: 'Open author workspace' })).toHaveAttribute(
    'href',
    `/submissions/${accepted.id}`,
  );
});

it('separates manuscript details from the review and supports keyboard tab navigation', async () => {
  renderWithApp(
    <SubmissionDetailPage id={accepted.id} />,
    fakeApi({ getSubmission: vi.fn(async () => accepted) }, author),
  );
  const review = await screen.findByRole('tab', { name: 'Review' });
  expect(review).toHaveAttribute('aria-selected', 'true');
  expect(screen.queryByText('AI disclosure:')).toBeNull();
  review.focus();
  await userEvent.keyboard('{ArrowLeft}');
  expect(screen.getByRole('tab', { name: 'Manuscript' })).toHaveFocus();
  expect(screen.getByRole('tabpanel')).toHaveAccessibleName('Manuscript');
  expect(screen.getByRole('heading', { name: 'Abstract' })).toBeVisible();
  expect(screen.queryByRole('region', { name: 'AI review report' })).toBeNull();
  await userEvent.keyboard('{End}');
  expect(screen.getByRole('tabpanel')).toHaveAccessibleName('Formalization');
});

it('offers an explicit formalization request without inventing a proposal', async () => {
  const paper = {
    ...accepted,
    analysis_visibility: 'public' as const,
    formalization: { items: [] },
  };
  const respondFormalization = vi.fn(async () => paper);
  renderWithApp(
    <SubmissionDetailPage id={paper.id} />,
    fakeApi({ getSubmission: vi.fn(async () => paper), respondFormalization }, author),
  );
  await userEvent.click(await screen.findByRole('tab', { name: 'Formalization' }));
  expect(screen.getByText('No formalization is scheduled.')).toBeVisible();
  expect(screen.queryByText('Awaiting your approval')).toBeNull();
  await userEvent.click(screen.getByText('Choose a statement to formalize'));
  await userEvent.click(screen.getAllByRole('button', { name: 'Request formalization' })[0]!);
  expect(respondFormalization).toHaveBeenCalledWith(paper.id, 'C1', { approve: true });
});
