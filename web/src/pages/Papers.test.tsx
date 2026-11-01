import userEvent from '@testing-library/user-event';
import { screen, within, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { paperSummary, publicPaper, SHA } from '../test/fixtures';
import { KATEX_ERROR, MACROS } from '../test/katex';
import { fakeApi, renderWithApp } from '../test/render';
import { ResultPage } from './Result';
import { PaperPage, PapersPage } from './Papers';

describe('public paper page', () => {
  it('prominently identifies confirmed errors on the published version', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({ ...publicPaper, confirmed_error_reports: 2 })),
    });
    renderWithApp(<PaperPage record="WP-2026-0001" />, api);
    expect(
      await screen.findByText(/This version has 2 confirmed error report/),
    ).toBeInTheDocument();
    expect(screen.getByText(/Read the corrections/)).toBeInTheDocument();
  });
  it('shows the statements with KaTeX and a Lean badge, and hides the analysis when absent', async () => {
    const api = fakeApi({ getPaper: vi.fn(async () => publicPaper) });
    renderWithApp(<PaperPage record="WP-2026-0001" />, api);

    expect(
      await screen.findByRole('heading', { level: 1, name: 'On sums of integers' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'arXiv:2609.33421' })).toHaveAttribute(
      'href',
      'https://arxiv.org/abs/2609.33421',
    );
    expect(screen.getByRole('link', { name: 'View PDF' })).toHaveAttribute(
      'href',
      '/api/v1/submissions/sub-acc/files/pdf?version=1',
    );

    const statements = screen
      .getByRole('heading', { name: 'Results' })
      .closest('section') as HTMLElement;
    expect(within(statements).getByRole('link', { name: 'Theorem 1.1' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001/results/C1',
    );
    expect(statements.querySelector('.katex')).not.toBeNull();
    expect(within(statements).getAllByText('A verified formal proof is available.')).toHaveLength(
      1,
    );

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
    renderWithApp(<ResultPage record="WP-2026-0001" claim="C1" />, api);
    expect(await screen.findByRole('heading', { name: 'Contribution' })).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'Conditional on these assumptions' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Conjecture 4.1' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001/results/C3',
    );
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
    const statements = (await screen.findByRole('heading', { name: 'Results' })).closest(
      'section',
    ) as HTMLElement;
    expect(within(statements).getAllByText('A verified formal proof is available.')).toHaveLength(
      1,
    );
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
    expect(screen.queryByRole('heading', { name: 'Results' })).not.toBeInTheDocument();
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
    await screen.findByRole('heading', { name: 'Results' });
    // The abstract and the result excerpt both use the publication macros.
    expect(container.querySelectorAll('.mop')).toHaveLength(2);
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
    expect(await screen.findByRole('heading', { name: 'Results' })).toBeInTheDocument();
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
  expect(await screen.findByRole('link', { name: '10.1000/example' })).toBeVisible();
  expect(screen.queryByText(/^arXiv:/)).toBeNull();
});

it('shows manuscript evidence as source links without granting another Lean verification badge', async () => {
  const api = fakeApi({
    getPaper: vi.fn(async () => ({
      ...publicPaper,
      origin: {
        problem_ids: ['problem-a', 'problem-b'],
        source_url: 'https://example.org/note',
        license: 'CC-BY-4.0',
        evidence: [
          {
            claim_label: 'Theorem 1',
            kind: 'lean_source' as const,
            url: `https://github.com/example/research/blob/${SHA}/Proof.lean`,
            source_revision: SHA,
            scope: 'Formalizes the finite case.',
          },
        ],
      },
    })),
  });
  renderWithApp(<PaperPage record="WP-2026-0001" />, api);
  expect(
    await screen.findByRole('link', { name: 'Original manuscript and sources' }),
  ).toHaveAttribute('href', 'https://example.org/note');
  expect(screen.getByRole('link', { name: 'Lean source' })).toHaveAttribute(
    'href',
    `https://github.com/example/research/blob/${SHA}/Proof.lean`,
  );
  expect(
    screen.getByText(
      'This record preserves the original problem association and the sources supplied with the manuscript.',
    ),
  ).toBeVisible();
  expect(screen.queryByText('Related problems: problem-a, problem-b')).toBeNull();
  expect(screen.getAllByText('A verified formal proof is available.')).toHaveLength(1);
});

it('provides source downloads and bibliographic metadata only for public publication inputs', async () => {
  renderWithApp(
    <PaperPage record={paperSummary.record} />,
    fakeApi({
      getPaper: vi.fn(async () => ({
        ...publicPaper,
        publication: { version: 1, claims_revision: 1 },
        versions: [
          { number: 1, uploaded_at: '2026-10-01T12:00:00Z', has_pdf: true, source_is_pdf: false },
        ],
      })),
    }),
  );
  expect(await screen.findByRole('link', { name: 'Download source' })).toHaveAttribute(
    'href',
    '/api/v1/submissions/sub-acc/files/source?version=1',
  );
  expect(document.querySelector('meta[name="citation_pdf_url"]')).toHaveAttribute(
    'content',
    `${window.location.origin}/api/v1/submissions/sub-acc/files/pdf?version=1`,
  );
  await userEvent.click(screen.getByText('Cite this paper'));
  await userEvent.selectOptions(screen.getByLabelText('Citation format'), 'bibtex');
  expect(screen.getByText(/@misc\{WP-2026-0001,/)).toBeVisible();
  expect(screen.getByRole('button', { name: 'Download BibTeX' })).toBeVisible();
});
