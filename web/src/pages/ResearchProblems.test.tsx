import { screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { ConjecturesPage } from './Conjectures';
import { ResearchProblemPage } from './ResearchProblems';
import type { ResearchProblem } from '../api/types';

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
