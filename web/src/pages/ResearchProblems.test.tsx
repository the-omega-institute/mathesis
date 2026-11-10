import { screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { ConjecturesPage } from './Conjectures';
import { ResearchProblemPage } from './ResearchProblems';
import type { ResearchProblem } from '../api/types';
import { setLocale } from '../i18n/locale';

const problem: ResearchProblem = {
  next_before: 'WP-2026-0001',
  summary: {
    id: 'stable-problem',
    source_id: 'original-id',
    title: 'A named conjecture',
    status: 'settled',
    notes: 1,
    updated_at: '2026-11-02T08:00:00Z',
  },
  descriptions: [
    {
      id: 'original-id',
      problem: { name: 'A named conjecture', source: { kind: 'oeis', locator: 'A123456' } },
      statement: 'The original mathematical question.',
      proposed_by: [{ name: 'Original Poser' }],
    },
  ],
  notes: [
    {
      record: 'WP-2026-0001',
      title: 'A readable short note',
      authors: [{ name: 'Note Author' }],
      kind: 'note',
      version: 2,
      upstream_version: 'r4',
      accepted_at: '2026-11-02T08:00:00Z',
      updated_at: '2026-11-02T08:00:00Z',
      source_url: 'https://example.org/note',
      license: 'CC-BY-4.0',
      results: [
        {
          claim: 'C1',
          kind: 'theorem',
          label: 'Theorem 1 [thm:main]',
          statement: 'A precise result.',
          outcome: 'proved',
        },
      ],
      evidence: [
        {
          claim_label: 'C1',
          kind: 'lean_source',
          url: 'https://github.com/example/research/blob/0123456789/Result.lean',
          scope: 'The main finite case',
        },
      ],
      contributions: [],
    },
  ],
};

const curated: ResearchProblem = {
  next_before: null,
  summary: {
    id: 'curated-problem',
    source_id: 'A391810',
    source: { kind: 'oeis', locator: 'A391810', year: 2025 },
    title: 'An alternating-sum formula in OEIS A391810',
    status: 'curated_open',
    status_as_of: '2026-11-10',
    notes: 0,
    updated_at: '2026-11-11T08:00:00Z',
  },
  descriptions: [],
  notes: [],
  collected_notes: [],
  curated: {
    name: 'An alternating-sum formula in OEIS A391810',
    statement: 'For every $n \\ge 1$, the alternating sum equals $b(n)$.',
    background: 'Conjectured from the first $200$ terms.',
    posers: [
      { name: 'First Poser', profile_url: 'https://example.org/~first' },
      { name: 'Second Poser' },
    ],
    source: { kind: 'oeis', locator: 'A391810', year: 2025 },
    status: 'open',
    status_as_of: '2026-11-10',
    status_evidence: [
      {
        url: 'https://oeis.org/A391810',
        checked_on: '2026-11-08',
        observed: 'The formula is marked as a conjecture.',
      },
      {
        url: 'javascript:alert(1)',
        checked_on: '2026-11-10',
        observed: 'The question has no accepted answer.',
      },
    ],
    msc: ['11B83'],
    directions: ['alternating-sums'],
    source_license: 'CC BY-SA 4.0 (OEIS End-User License Agreement)',
    license: 'CC-BY-SA-4.0',
    version: 2,
    published_on: '2026-11-11T08:00:00Z',
  },
};

describe('directly collected curated problems', () => {
  it('lists a curated problem under its dated curator reading before any work names it', async () => {
    renderWithApp(
      <ConjecturesPage />,
      fakeApi({
        listResearchProblems: vi.fn(async () => ({
          items: [
            curated.summary,
            {
              ...curated.summary,
              id: 'partly',
              title: 'A partly resolved question',
              status: 'curated_partially_resolved' as const,
              notes: 2,
            },
          ],
          next_before: null,
        })),
      }),
    );
    const open = await screen.findByRole('link', {
      name: 'An alternating-sum formula in OEIS A391810',
    });
    expect(open).toHaveAttribute('href', '/problems/curated-problem');
    expect(open.closest('li')).toHaveTextContent(
      'Curated · Open as of 10 November 2026 · 0 papers and notes',
    );
    expect(
      screen.getByRole('link', { name: 'A partly resolved question' }).closest('li'),
    ).toHaveTextContent('Curated · Partially resolved as of 10 November 2026 · 2 papers and notes');
    expect(screen.queryByText(/Settled|Reviewed research/)).not.toBeInTheDocument();
  });

  it('shows the statement, posers, source and dated evidence as a curator reading with ways to take part', async () => {
    renderWithApp(
      <ResearchProblemPage id="curated-problem" />,
      fakeApi({ getResearchProblem: vi.fn(async () => curated) }),
    );
    expect(
      await screen.findByRole('heading', {
        level: 1,
        name: 'An alternating-sum formula in OEIS A391810',
      }),
    ).toBeInTheDocument();
    // The statement is rendered as mathematics, not shown as LaTeX source.
    const statement = screen.getByRole('region', { name: 'Problem statement' });
    expect(statement.querySelector('.katex')).not.toBeNull();
    expect(statement).toHaveTextContent('the alternating sum equals');
    expect(statement).toHaveTextContent('Proposed by First Poser; Second Poser');
    expect(screen.getByRole('link', { name: 'First Poser' })).toHaveAttribute(
      'href',
      'https://example.org/~first',
    );
    expect(screen.getByRole('link', { name: 'OEIS A391810 (2025)' })).toHaveAttribute(
      'href',
      'https://oeis.org/A391810',
    );
    expect(statement).toHaveTextContent('MSC 11B83 · alternating-sums');
    expect(screen.getByText(/Conjectured from the first/)).toBeInTheDocument();

    const status = screen.getByRole('region', { name: 'Open as of 10 November 2026' });
    expect(status).toHaveTextContent(
      'This status is the curator’s reading of the sources below on the dates shown. It is not a Mathesis review or audit result.',
    );
    const evidence = status.querySelectorAll('li');
    expect(evidence).toHaveLength(2);
    expect(evidence[0]).toHaveTextContent(
      'https://oeis.org/A391810 · Checked on 8 November 2026 · The formula is marked as a conjecture.',
    );
    expect(screen.getByRole('link', { name: 'https://oeis.org/A391810' })).toHaveAttribute(
      'href',
      'https://oeis.org/A391810',
    );
    // An address that is not a web page is shown as text, never as a link.
    expect(evidence[1]).toHaveTextContent('javascript:alert(1) · Checked on 10 November 2026');
    expect(evidence[1]!.querySelector('a')).toBeNull();

    expect(screen.getByRole('link', { name: 'Submit a written solution' })).toHaveAttribute(
      'href',
      '/submit',
    );
    expect(screen.getByRole('link', { name: 'Contribute a Lean formalization' })).toHaveAttribute(
      'href',
      '/submit?type=lean&problem=curated-problem',
    );
    expect(screen.getByText('No paper or note addresses this problem yet.')).toBeInTheDocument();
    expect(screen.getByText(/Statement text: CC-BY-SA-4.0/)).toHaveTextContent(
      'Statement text: CC-BY-SA-4.0 · Source licence: CC BY-SA 4.0 (OEIS End-User License Agreement) · Version 2 · 11 November 2026',
    );
    expect(screen.queryByText(/Settled|Reviewed research|Lean ✓|verified/i)).toBeNull();
  });

  it('states the dated curator reading in Simplified Chinese', async () => {
    setLocale('zh');
    try {
      renderWithApp(
        <ResearchProblemPage id="curated-problem" />,
        fakeApi({ getResearchProblem: vi.fn(async () => curated) }),
      );
      const status = await screen.findByRole('region', { name: '截至 2026年11月10日 未解决' });
      expect(status).toHaveTextContent('不是 Mathesis 的审阅或审核结论');
      expect(status).toHaveTextContent('查阅于 2026年11月8日');
      expect(screen.getByText(/^整理收录 · 截至 2026年11月10日 未解决$/)).toBeInTheDocument();
      expect(screen.getByRole('link', { name: '贡献 Lean 形式化' })).toHaveAttribute(
        'href',
        '/submit?type=lean&problem=curated-problem',
      );
    } finally {
      setLocale('en');
    }
  });

  it('keeps the curator reading when collected works name the problem and credits an unknown poser honestly', async () => {
    const named: ResearchProblem = {
      ...curated,
      summary: { ...curated.summary, status: 'curated_partially_resolved', notes: 1 },
      curated: {
        ...curated.curated!,
        status: 'partially_resolved',
        posers: [],
        posers_unknown: 'The source page names no author.',
      },
      collected_notes: [
        {
          work: 'a-collected-note',
          title: 'A proof for even n',
          authors: [{ name: 'Collected Author' }],
          version: 1,
          published_on: '2026-11-11T08:00:00Z',
          license: 'CC-BY-4.0',
          public_path: '/papers/a-collected-note?version=1',
        },
      ],
    };
    renderWithApp(
      <ResearchProblemPage id="curated-problem" />,
      fakeApi({
        getResearchProblem: vi.fn(async () => named),
        collectedKnowledge: vi.fn(async () => ({
          work: 'a-collected-note',
          version: 1,
          statements: [],
          connections: [],
          macros: {},
          limitations: [],
          formalizations: [],
        })),
      }),
    );
    expect(
      await screen.findByRole('region', { name: 'Partially resolved as of 10 November 2026' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText('Original poser not recorded: The source page names no author.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'A proof for even n' })).toHaveAttribute(
      'href',
      '/papers/a-collected-note?version=1',
    );
    expect(screen.queryByText('No paper or note addresses this problem yet.')).toBeNull();
    expect(screen.queryByText(/Settled/)).toBeNull();
  });
});

describe('problem-centred research records', () => {
  it('starts with stable research records and links a problem independently of its note', async () => {
    const listConjectures = vi.fn(async () => ({ items: [], next_before: null }));
    renderWithApp(
      <ConjecturesPage />,
      fakeApi({
        listResearchProblems: vi.fn(async () => ({ items: [problem.summary], next_before: null })),
        listConjectures,
      }),
    );
    expect(await screen.findByRole('link', { name: 'A named conjecture' })).toHaveAttribute(
      'href',
      '/problems/stable-problem',
    );
    expect(screen.getByText(/Settled · 1 paper or note/)).toBeInTheDocument();
    expect(listConjectures).not.toHaveBeenCalled();
  });

  it('keeps original problem credit, note authors, version and external Lean evidence distinct', async () => {
    renderWithApp(
      <ResearchProblemPage id="stable-problem" />,
      fakeApi({ getResearchProblem: vi.fn(async () => problem) }),
    );
    expect(await screen.findByRole('heading', { name: 'A named conjecture' })).toBeInTheDocument();
    expect(screen.getByText('Original Poser').parentElement).toHaveTextContent(
      'Proposed by Original Poser',
    );
    expect(screen.getByText('Note Author')).toBeInTheDocument();
    expect(screen.getByText(/Version 2/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'A readable short note' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001',
    );
    expect(screen.getByRole('link', { name: 'Theorem 1' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001/results/C1',
    );
    expect(screen.getByRole('link', { name: 'The main finite case' })).toHaveAttribute(
      'href',
      problem.notes[0]!.evidence[0]!.url,
    );
    expect(screen.getByRole('link', { name: 'More papers and notes' })).toHaveAttribute(
      'href',
      '/problems/stable-problem?before=WP-2026-0001',
    );
    expect(screen.queryByText(/Lean verified|Lean ✓|C1|r4|thm:main/)).not.toBeInTheDocument();
  });

  it('centres a problem named by collected works on its source, their exact version and Lean status', async () => {
    const lean = 'https://github.com/example/research/blob/0123456789/Formula.lean';
    const collected: ResearchProblem = {
      next_before: 'a-collected-note',
      summary: {
        id: 'collected-problem',
        source_id: 'A391810',
        source: { kind: 'oeis', locator: 'A391810' },
        title: "A poser's formula (5) in OEIS A391810",
        status: 'related_research',
        notes: 26,
        updated_at: '2026-11-10T08:00:00Z',
      },
      descriptions: [],
      notes: [],
      collected_notes: [
        {
          work: 'a-collected-note',
          title: 'A proof of the formula',
          authors: [{ name: 'Collected Author' }],
          version: 4,
          published_on: '2026-11-10T08:00:00Z',
          license: 'CC-BY-4.0',
          public_path: '/papers/a-collected-note?version=4',
          lean: { url: lean, revision: '0123456789' },
        },
      ],
    };
    const proof = { version: 4, next_step: '', proofs: [] };
    const collectedKnowledge = vi.fn(async () => ({
      work: 'a-collected-note',
      version: 4,
      statements: [],
      connections: [],
      macros: {},
      limitations: [],
      formalizations: [
        { ...proof, claim: 's1', state: 'verified' as const },
        { ...proof, claim: 's2', state: 'verified' as const },
        { ...proof, claim: 's3', state: 'open' as const },
        { ...proof, claim: 's4', state: 'preparing' as const },
      ],
    }));
    renderWithApp(
      <ResearchProblemPage id="collected-problem" />,
      fakeApi({ getResearchProblem: vi.fn(async () => collected), collectedKnowledge }),
    );
    expect(
      await screen.findByRole('heading', { name: "A poser's formula (5) in OEIS A391810" }),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'OEIS A391810' })).toHaveAttribute(
      'href',
      'https://oeis.org/A391810',
    );
    expect(screen.getByRole('link', { name: 'A proof of the formula' })).toHaveAttribute(
      'href',
      '/papers/a-collected-note?version=4',
    );
    expect(screen.getByText('Collected Author')).toBeInTheDocument();
    expect(screen.getByText(/Version 4/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Lean source' })).toHaveAttribute('href', lean);
    const status = (await screen.findByText(/Verified in Lean: 2/)).closest('p')!;
    expect(status).toHaveTextContent(
      'Lean status: Lean source · Verified in Lean: 2 · Open for formalization: 1',
    );
    expect(status).not.toHaveTextContent(/Disproved|Awaiting/);
    expect(collectedKnowledge).toHaveBeenCalledWith('a-collected-note', '4', expect.anything());
    expect(screen.getByRole('link', { name: 'More papers and notes' })).toHaveAttribute(
      'href',
      '/problems/collected-problem?before=a-collected-note',
    );
  });
});
