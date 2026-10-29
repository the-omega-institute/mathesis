import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ApiError, createApiClient } from '../api/client';
import { authorStanding } from '../lib/authorStanding';
import { authorStandingCases } from '../test/authorStanding';
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
import { feedbackLetter, queuedRound, refereeFile } from '../test/referee';

it.each([author, editor])(
  'keeps an in-review author page focused, including for $id',
  async (person) => {
    const api = fakeApi(
      {
        getSubmission: vi.fn(async () => ({ ...submission, submitter: person.id })),
        referee: vi.fn(async () =>
          refereeFile({ rounds: [queuedRound], letters: [{ ...feedbackLetter, round: 0 }] }),
        ),
      },
      person,
    );
    renderWithApp(<SubmissionDetailPage id={submission.id} />, api);
    const standing = await screen.findByRole('region', { name: 'Paper standing' });
    await waitFor(() => expect(api.referee).toHaveBeenCalled());
    expect(standing).toHaveTextContent('The final report will appear here');
    expect(standing).not.toHaveTextContent('Elapsed');
    expect(screen.getByRole('link', { name: 'View PDF' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Abstract' })).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Statements' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Final review report' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Upload a revised version' })).toBeNull();
    expect(screen.queryByRole('form', { name: 'Upload a new version', hidden: true })).toBeNull();
    for (const text of [
      'Details: evidence, history and settings',
      'Earlier reports',
      'Public binding discussions',
      'Authorized knowledge proposals',
      'Private research goals',
    ]) {
      expect(screen.queryByText(text)).toBeNull();
    }
    for (const call of [
      api.bindings,
      api.research,
      api.feedbacks,
      api.getAnalysis,
      api.previewDecision,
      api.getCma,
    ]) {
      expect(call).not.toHaveBeenCalled();
    }
    await userEvent.click(screen.getByText('Withdraw submission'));
    expect(screen.getByRole('button', { name: 'Withdraw paper…' })).toBeVisible();
  },
);

it.each(authorStandingCases)(
  'keeps the detail badge and standing headline equal for $paper.status.state / $paper.kind',
  async ({ paper, review }) => {
    const { container } = renderWithApp(
      <SubmissionDetailPage id={paper.id} />,
      fakeApi(
        {
          getSubmission: vi.fn(async () => paper),
          referee: vi.fn(async () => review),
        },
        author,
      ),
    );
    const card = await screen.findByRole('region', { name: 'Paper standing' });
    const standing = authorStanding(paper, review);
    const headline = await within(card).findByRole('heading', { name: standing.headline });
    const header = container.querySelector('.record-head')!;
    expect(header.querySelector('.badge')?.textContent).toBe(headline.textContent);
    expect(header.querySelector('.badge')?.textContent).toBe(standing.headline);
    expect(within(header as HTMLElement).queryByText('Private review')).toBeNull();
  },
);

describe('paper workspace', () => {
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

    const form = await screen.findByRole('form', { name: 'Confirm statements' });
    expect(screen.queryByRole('region', { name: 'Review' })).toBeNull();
    expect(screen.getByRole('heading', { name: 'Check your manuscript' })).toBeInTheDocument();
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
    await waitFor(() =>
      expect(screen.queryByRole('form', { name: 'Confirm statements' })).toBeNull(),
    );
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
    const form = await screen.findByRole('form', { name: 'Confirm statements' });
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

    const section = await screen.findByRole('region', { name: 'Paper standing' });
    expect(
      within(section).getByText('Theorem 1.1 already follows from 1901.00001.'),
    ).toBeInTheDocument();
    expect(
      within(section).getByText(
        'The main results follow from existing results without establishing a distinct new mathematical contribution.',
      ),
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
    await user.click(screen.getByRole('radio', { name: /^Public/ }));
    expect(setVisibility).toHaveBeenCalledWith('sub-acc', 'public');

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
    renderWithApp(<SubmissionDetailPage id="sub-1" staffView />, api);
    await userEvent.click(await screen.findByText('Details: evidence, history and settings'));
    expect(await screen.findByRole('heading', { name: 'Editor tools' })).toBeInTheDocument();
    expect(screen.getByText('Pending — awaiting S2 Literature')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Apply decision' }));
    expect(await screen.findByText('awaiting S2: S2 needs a human report')).toBeInTheDocument();
  });
});

it.each([
  draft,
  submission,
  { ...submission, purpose: 'review_only' as const, status: { state: 'review_complete' as const } },
  accepted,
  notAccepted,
  { ...submission, status: { state: 'withdrawn' as const } },
])('keeps raw identifiers out of author text and titles for $status.state', async (paper) => {
  const { refereeRound, refereeFile, feedbackLetter, refereeAudit } =
    await import('../test/referee');
  const round = {
    ...refereeRound,
    review_method: 'trureturing-content-2026-10-15',
    source_sha256: 'b'.repeat(64),
    referee: { ...refereeRound.referee, engine: 'nyxid-oracle', model: 'internal-model-id' },
    audit: {
      ...refereeRound.audit!,
      engine: 'nyxid-cma',
      state: {
        state: 'done' as const,
        at: refereeRound.started_at,
        result: {
          ...refereeAudit,
          claims: [
            {
              ...refereeAudit.claims[0]!,
              correctness: 'not_checked' as const,
              shape: 'bind_only' as const,
            },
          ],
        },
      },
    },
  };
  const { container } = renderWithApp(
    <SubmissionDetailPage id={paper.id} />,
    fakeApi(
      {
        getSubmission: vi.fn(async () => paper),
        referee: vi.fn(async () =>
          refereeFile({
            rounds: [round],
            letters: [feedbackLetter],
            cma_attempts: [
              {
                round: 1,
                kind: 'audit',
                step: 'r1:audit',
                attempt: 1,
                payer: 'operator',
                reason: 'operator_step',
                status: 'succeeded',
                admitted_at: refereeRound.started_at,
                finished_at: refereeRound.started_at,
              },
            ],
          }),
        ),
      },
      author,
    ),
  );
  // Draft and withdrawn pages carry no review workspace; open every fold that exists.
  const reviewShown = !['draft', 'withdrawn', 'in_review'].includes(paper.status.state);
  if (reviewShown) await screen.findByRole('region', { name: 'Final review report' });
  else await screen.findByRole('region', { name: 'Paper standing' });
  for (const label of ['Manuscript details', 'Results in this manuscript']) {
    const toggle = screen.queryByText(label);
    if (toggle) await userEvent.click(toggle);
  }
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  const textNodes: string[] = [];
  while (walker.nextNode()) textNodes.push(walker.currentNode.textContent ?? '');
  const text = [
    ...textNodes,
    ...Array.from(container.querySelectorAll('[title]'), (element) =>
      element.getAttribute('title'),
    ),
    ...Array.from(container.querySelectorAll('[aria-label]'), (element) =>
      element.getAttribute('aria-label'),
    ),
  ].join('\n');
  for (const raw of [
    'trureturing-content-2026-10-15',
    'nyxid-',
    'internal-model-id',
    'bind_only',
    'not_checked',
    'escape witness',
    'CMA',
    'Codex',
    'payer',
    'Oracle',
    'GPT',
    'S0',
    'S1',
    'S2',
    'S3',
    'contract',
    'workspace id',
    'bind-only',
  ])
    expect(text.toLowerCase()).not.toContain(raw.toLowerCase());
  if (!reviewShown) return;
  expect(text).toContain('Not checked');
  expect(text).not.toContain('Follows from known results');
  expect(text).not.toContain('Source check');
  for (const label of [
    'Advanced: compute settings and history',
    'Public binding discussions',
    'Authorized knowledge proposals',
    'Private research goals',
    'SHA-256',
    'Elapsed since',
  ])
    expect(text).not.toContain(label);
  expect(
    screen.getByRole('region', {
      name: paper.status.state === 'in_review' ? 'Review feedback' : 'Final review report',
    }),
  ).not.toHaveTextContent('b'.repeat(64));
  expect(screen.getByText('Manuscript details').closest('details')).not.toHaveTextContent(
    'b'.repeat(64),
  );
});

it.each(['submission', 'review'] as const)(
  'keeps author-facing %s network errors free of retry advice',
  async (target) => {
    const fetchImpl = vi.fn(async () => {
      throw new TypeError('Failed to fetch');
    });
    const networkApi = createApiClient(fetchImpl);
    renderWithApp(
      <SubmissionDetailPage id={submission.id} />,
      fakeApi(
        {
          getSubmission:
            target === 'submission' ? networkApi.getSubmission : vi.fn(async () => submission),
          referee: networkApi.referee,
        },
        author,
      ),
    );
    if (target === 'review') {
      const card = await screen.findByRole('region', { name: 'Paper standing' });
      await waitFor(() => expect(fetchImpl).toHaveBeenCalledOnce());
      expect(within(card).getByRole('status')).toHaveTextContent(
        'The final report will appear here',
      );
      expect(screen.queryByRole('alert')).toBeNull();
      expect(screen.queryByRole('button', { name: 'Upload a revised version' })).toBeNull();
      return;
    }
    const notices = await screen.findAllByRole('alert');
    for (const notice of notices) {
      expect(notice).toHaveTextContent('The server could not be reached.');
      expect(notice.textContent).not.toMatch(/try again/i);
    }
    await userEvent.click(within(notices[0]!).getByRole('button', { name: 'Reload page data' }));
    await waitFor(() => expect(fetchImpl).toHaveBeenCalledTimes(2));
    for (const notice of await screen.findAllByRole('alert'))
      expect(notice.textContent).not.toMatch(/try again/i);
  },
);

it('shows one loading shell for the shared review resource on the author page', async () => {
  const { container } = renderWithApp(
    <SubmissionDetailPage id={accepted.id} />,
    fakeApi(
      {
        getSubmission: vi.fn(async () => accepted),
        referee: vi.fn(() => new Promise<never>(() => undefined)),
      },
      author,
    ),
  );
  await screen.findByRole('region', { name: 'Paper standing' });
  await userEvent.click(screen.getByText('Manuscript details'));
  expect(screen.getAllByText('Loading…')).toHaveLength(1);
  expect(container.querySelector('.author-details .referee-panel')).toBeNull();
  expect(screen.getByRole('region', { name: 'Review' })).toHaveTextContent('Loading…');
});
