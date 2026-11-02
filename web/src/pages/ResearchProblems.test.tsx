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
});
