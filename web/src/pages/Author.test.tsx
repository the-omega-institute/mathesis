import { screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ConjectureSummary, FormalizationOpportunity } from '../api/types';
import { author, paperSummary } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { AuthorPage } from './Author';

describe('AuthorPage', () => {
  it('shows an author profile with papers, conjectures and formalization work', async () => {
    const conjecture: ConjectureSummary = {
      claim: 'C2',
      source: 'from WP-2026-0001',
      status: 'open',
      attempts: 0,
      solver: null,
      record: 'WP-2026-0001',
      title: 'An open bound',
      statement: 'For all $n$, $n = n$.',
      lean_statement_status: 'confirmed',
      authors: [{ name: author.display_name, affiliation: 'University of Example' }],
    };
    const opportunity: FormalizationOpportunity = {
      record: paperSummary.record,
      claim: 'C1',
      title: paperSummary.title,
      label: 'Theorem 1.1',
      statement: 'Every sum has the stated value.',
      authors: [{ name: author.display_name }],
      kind: 'paper',
      status: 'verified',
      target_digest: 'a'.repeat(64),
      toolchain: 'Lean',
      verified_count: 1,
      contributor: null,
      attribution: null,
      receipt: null,
    };
    renderWithApp(
      <AuthorPage name={author.display_name} />,
      fakeApi(
        {
          listPapers: vi.fn(async () => ({ items: [paperSummary], next_before: null })),
          listConjectures: vi.fn(async ({ status }) => ({
            items: status === 'open' ? [conjecture] : [],
            next_before: null,
          })),
          formalizationOpportunities: vi.fn(async () => ({
            items: [opportunity],
            next_before: null,
          })),
        },
        author,
      ),
    );
    expect(await screen.findByRole('heading', { name: author.display_name })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Papers and notes' })).toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: paperSummary.title })[0]).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001',
    );
    expect(screen.getByRole('heading', { name: 'Conjectures' })).toBeInTheDocument();
    expect(screen.getByLabelText('Author contribution counts')).toHaveTextContent(
      '1 papers and notes',
    );
    expect(screen.getByRole('heading', { name: 'Formalization' })).toBeInTheDocument();
  });

  it('opens the same profile for a saved link in either written form of the name', async () => {
    const shown = [{ name: 'Haobo Ma', affiliation: 'Institute of Example' }];
    const conjecture: ConjectureSummary = {
      claim: 'C2',
      source: 'from WP-2026-0001',
      status: 'open',
      attempts: 0,
      solver: null,
      record: 'WP-2026-0001',
      title: 'An open bound',
      statement: 'For all $n$, $n = n$.',
      lean_statement_status: 'confirmed',
      authors: shown,
    };
    for (const name of ['Ma, Haobo', 'Haobo Ma']) {
      const api = fakeApi({
        listPapers: vi.fn(async () => ({
          items: [{ ...paperSummary, authors: shown }],
          next_before: null,
        })),
        listConjectures: vi.fn(async ({ status }) => ({
          items: status === 'open' ? [conjecture] : [],
          next_before: null,
        })),
      });
      const page = renderWithApp(<AuthorPage name={name} />, api);
      expect(await screen.findByRole('heading', { name: 'Haobo Ma' })).toBeInTheDocument();
      expect(screen.getByText('Institute of Example')).toBeInTheDocument();
      expect(screen.getByLabelText('Author contribution counts')).toHaveTextContent(
        '1 papers and notes · 1 conjectures',
      );
      expect(api.formalizationOpportunities).toHaveBeenCalledWith(
        { q: 'Haobo Ma', limit: 100 },
        expect.any(Object),
      );
      page.unmount();
    }
  });
});
