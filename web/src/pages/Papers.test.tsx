import userEvent from '@testing-library/user-event';
import { screen, within, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { paperSummary, publicPaper, SHA } from '../test/fixtures';
import { KATEX_ERROR, MACROS } from '../test/katex';
import { fakeApi, renderWithApp } from '../test/render';
import { PaperPage, PapersPage } from './Papers';

describe('public paper page', () => {
  it('prominently identifies confirmed errors on the published version', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({ ...publicPaper, confirmed_error_reports: 2 })),
    });
    renderWithApp(<PaperPage record="WP-2026-0001" />, api);
    expect(
      await screen.findByText(/This published version has 2 confirmed error report/),
    ).toBeInTheDocument();
    expect(screen.getByText(/acceptance is not a proof of correctness/)).toBeInTheDocument();
  });
  it('shows the statements with KaTeX and a Lean badge, and hides the analysis when absent', async () => {
    const api = fakeApi({ getPaper: vi.fn(async () => publicPaper) });
    renderWithApp(<PaperPage record="WP-2026-0001" />, api);

    expect(
      await screen.findByRole('heading', { level: 1, name: 'On sums of integers' }),
    ).toBeInTheDocument();
    expect(screen.getByText('WP-2026-0001')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'doi:10.48550/arXiv.2609.33421' })).toHaveAttribute(
      'href',
      'https://doi.org/10.48550/arXiv.2609.33421',
    );
    expect(screen.getByRole('link', { name: 'PDF (version 1)' })).toHaveAttribute(
      'href',
      '/api/v1/submissions/sub-acc/files/pdf?version=1',
    );

    const statements = screen
      .getByRole('heading', { name: 'Statements' })
      .closest('section') as HTMLElement;
    expect(within(statements).getByText('C1')).toBeInTheDocument();
    expect(statements.querySelector('.katex')).not.toBeNull();
    // Only C1 carries a Lean proof.
    expect(within(statements).getAllByText('Lean ✓')).toHaveLength(1);
    expect(
      within(statements).getByRole('link', {
        name: `ada/sums-lean@${SHA.slice(0, 12)}`,
        hidden: true,
      }),
    ).toHaveAttribute('href', `https://github.com/ada/sums-lean/tree/${SHA}`);

    expect(screen.queryByRole('heading', { name: 'Analysis' })).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Conjecture follow-ups' })).toBeNull();
  });

  it('shows dependencies and new lemmas without review labels or commentary', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({
        ...publicPaper,
        claims: publicPaper.claims!.map((c) => ({ ...c, depends_on: c.id === 'C1' ? ['C3'] : [] })),
        new_content: [{ claim: 'C1', lemmas: ['A new intermediate identity $x = y$.'] }],
      })),
    });
    renderWithApp(<PaperPage record="WP-2026-0001" />, api);
    expect(await screen.findByRole('heading', { name: 'New in this paper' })).toBeInTheDocument();
    expect(screen.getByText('Uses C3.')).toBeInTheDocument();
    expect(screen.getByText(/A new intermediate identity/)).toBeInTheDocument();
    for (const text of [
      'Correct',
      'Gap',
      'Error',
      'Not checked',
      'Analysis',
      'Escape witnesses',
      'Confirmed by an editor',
    ]) {
      expect(screen.queryByText(text)).not.toBeInTheDocument();
    }
  });

  it('marks only verified statements with Lean when the API sends null', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({
        ...publicPaper,
        claims: publicPaper.claims!.map((c) => (c.lean ? c : { ...c, lean: null })),
      })),
    });
    renderWithApp(<PaperPage record="WP-2026-0001" />, api);
    const statements = (await screen.findByRole('heading', { name: 'Statements' })).closest(
      'section',
    ) as HTMLElement;
    expect(within(statements).getAllByText('Lean ✓')).toHaveLength(1);
  });

  it('renders a private record with only its title, authors, kind and record', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({
        summary: {
          kind: 'note' as const,
          record: 'WP-2026-0002',
          title: 'A short note',
          authors: [{ name: 'Ada Author' }],
        },
      })),
    });
    renderWithApp(<PaperPage record="WP-2026-0002" />, api);
    expect(await screen.findByRole('heading', { name: 'A short note' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Statements' })).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Abstract' })).not.toBeInTheDocument();
  });

  it('renders statements and the abstract with the paper’s macros', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({
        ...publicPaper,
        summary: { ...publicPaper.summary, abstract_text: 'We study $\\rep(G)$.' },
        claims: [{ ...publicPaper.claims![0]!, statement: 'The map $\\rep$ is \\emph{onto}.' }],
        macros: MACROS,
      })),
    });
    const { container } = renderWithApp(<PaperPage record="WP-2026-0001" />, api);
    await screen.findByRole('heading', { name: 'Statements' });
    // The abstract, the row preview and the expandable statement body.
    expect(container.querySelectorAll('.mop')).toHaveLength(3);
    expect(container.querySelector(KATEX_ERROR)).toBeNull();
  });

  it('renders a missing macros field as no macros', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({
        ...publicPaper,
        macros: null as unknown as Record<string, string>,
      })),
    });
    renderWithApp(<PaperPage record="WP-2026-0001" />, api);
    expect(await screen.findByRole('heading', { name: 'Statements' })).toBeInTheDocument();
  });

  it('lists accepted papers', async () => {
    const api = fakeApi({
      listPapers: vi.fn(async () => ({ items: [paperSummary], next_before: null })),
    });
    renderWithApp(<PapersPage />, api);
    expect(await screen.findByRole('link', { name: 'On sums of integers' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001',
    );
    expect(screen.getByText('Lean ✓ 1')).toBeInTheDocument();
  });
});

it('searches the complete archive through the API and resets pagination when filters change', async () => {
  const listPapers = vi.fn(async () => ({ items: [paperSummary], next_before: 'WP-2026-0001' }));
  renderWithApp(<PapersPage />, fakeApi({ listPapers }));
  const user = userEvent.setup();
  await screen.findByText(paperSummary.title);
  await user.type(screen.getByLabelText('Find research'), 'prime gaps');
  await user.type(screen.getByLabelText('Author'), 'A. Author');
  await user.type(screen.getByLabelText('Subject (MSC)'), '11');
  await user.selectOptions(screen.getByLabelText('Type'), 'paper');
  await user.click(screen.getByRole('button', { name: 'Search' }));
  await waitFor(() =>
    expect(listPapers).toHaveBeenLastCalledWith(
      { q: 'prime gaps', author: 'A. Author', msc: '11', kind: 'paper', before: null, limit: 25 },
      expect.any(Object),
    ),
  );
  await user.click(screen.getByRole('button', { name: 'Clear filters' }));
  await waitFor(() =>
    expect(listPapers).toHaveBeenLastCalledWith({ before: null, limit: 25 }, expect.any(Object)),
  );
});

it('shows an abstract excerpt on listing cards and the complete public version history', async () => {
  const api = fakeApi({
    getPaper: vi.fn(async () => ({
      ...publicPaper,
      versions: [
        { number: 1, uploaded_at: '2026-10-01T12:00:00Z', has_pdf: true },
        { number: 2, uploaded_at: '2026-10-10T12:00:00Z', has_pdf: false },
        { number: 3, uploaded_at: '2026-10-22T12:00:00Z', has_pdf: true },
      ],
    })),
  });
  renderWithApp(<PaperPage record={paperSummary.record} />, api);
  expect(await screen.findByText('arXiv:2609.33421')).toBeVisible();
  const history = screen.getByRole('heading', { name: 'Version history' }).closest('section')!;
  expect(within(history).getAllByRole('listitem')).toHaveLength(3);
  expect(within(history).getByRole('link', { name: 'Version 1 PDF' })).toHaveAttribute(
    'href',
    '/api/v1/submissions/sub-acc/files/pdf?version=1',
  );
  expect(within(history).getByRole('link', { name: 'Version 3 PDF' })).toHaveAttribute(
    'href',
    '/api/v1/submissions/sub-acc/files/pdf?version=3',
  );
  expect(within(history).queryByRole('link', { name: 'Version 2 PDF' })).toBeNull();
  expect(history.querySelectorAll('time')).toHaveLength(3);
});

it('keeps a searchable listing useful with abstract excerpts and preserves minimal legacy cards', async () => {
  const abstract = 'We prove a useful estimate for sums of integers. '.repeat(20);
  renderWithApp(
    <PapersPage />,
    fakeApi({
      listPapers: vi.fn(async () => ({
        items: [
          { ...paperSummary, abstract_text: abstract },
          {
            kind: 'note' as const,
            record: 'WP-2026-0002',
            title: 'Legacy note',
            authors: [{ name: 'Another Author' }],
          },
        ],
        next_before: null,
      })),
    }),
  );
  const title = await screen.findByRole('link', { name: paperSummary.title });
  const card = title.closest('article')!;
  expect(card.querySelector('.abstract-excerpt')).toHaveTextContent(/^We prove a useful estimate/);
  expect(card.querySelector('.abstract-excerpt')).toHaveTextContent(abstract.trim());
  expect(card.querySelector('time')).not.toBeNull();
  expect(
    screen
      .getByRole('link', { name: 'Legacy note' })
      .closest('article')
      ?.querySelector('.abstract-excerpt'),
  ).toBeNull();
});

it('renders complete math across the former 400-character excerpt boundary', async () => {
  const formula = String.raw`\frac{n(n+1)}{2} = \sum_{k=1}^{n} k`;
  const abstract = `${'A useful estimate. '.repeat(20)}$${formula}$ holds for all n. ${'More context. '.repeat(30)}`;
  expect(abstract.indexOf('$')).toBeLessThan(400);
  expect(abstract.indexOf('$', abstract.indexOf('$') + 1)).toBeGreaterThan(400);
  renderWithApp(
    <PapersPage />,
    fakeApi({
      listPapers: vi.fn(async () => ({
        items: [{ ...paperSummary, abstract_text: abstract }],
        next_before: null,
      })),
    }),
  );
  const card = (await screen.findByRole('link', { name: paperSummary.title })).closest('article')!;
  const excerpt = card.querySelector('.abstract-excerpt')!;
  expect(excerpt.querySelector('.katex')).not.toBeNull();
  expect(excerpt.querySelector('annotation[encoding="application/x-tex"]')).toHaveTextContent(
    formula,
  );
  expect(excerpt.querySelector(KATEX_ERROR)).toBeNull();
  expect(excerpt).toHaveTextContent('holds for all n.');
});

it('does not derive an arXiv identifier from an unrelated DOI', async () => {
  renderWithApp(
    <PaperPage record={paperSummary.record} />,
    fakeApi({
      getPaper: vi.fn(async () => ({
        ...publicPaper,
        summary: { ...paperSummary, doi: '10.1000/example' },
      })),
    }),
  );
  expect(await screen.findByRole('link', { name: 'doi:10.1000/example' })).toBeVisible();
  expect(screen.queryByText(/^arXiv:/)).toBeNull();
});
