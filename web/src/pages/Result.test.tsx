import { screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { publicPaper, claims } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { ResultPage } from './Result';

describe('public result', () => {
  it('makes an accepted result citable without a Lean target and credits the original authors', async () => {
    const api = fakeApi({
      getPaper: vi.fn(async () => ({
        ...publicPaper,
        publication: { version: 1, claims_revision: 2 },
        claims: claims.map((claim) => ({ ...claim, lean: null })),
      })),
    });
    renderWithApp(<ResultPage record="WP-2026-0001" claim="C1" />, api);
    expect(await screen.findByRole('heading', { name: 'Theorem 1.1', level: 1 })).toBeVisible();
    expect(screen.getByText('No public formal proof is available for this result.')).toBeVisible();
    expect(screen.getByRole('link', { name: 'Ada Author' })).toHaveAttribute(
      'href',
      '/papers?author=Ada+Author',
    );
    expect(screen.getByText('Cite this result')).toBeVisible();
    expect(api.getStatement).not.toHaveBeenCalled();
    expect(screen.getByRole('link', { name: 'Lemma 2.1' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001/results/C2',
    );
    expect(screen.getByText('Dependencies stated in the manuscript.')).toBeVisible();
  });

  it('does not reveal a theorem or cite it when the public projection is private', async () => {
    const api = fakeApi({ getPaper: vi.fn(async () => ({ summary: publicPaper.summary })) });
    renderWithApp(<ResultPage record="WP-2026-0001" claim="C1" />, api);
    expect(await screen.findByText('This result is not available publicly.')).toBeVisible();
    expect(screen.queryByText('Cite this result')).toBeNull();
    expect(screen.queryByText('Theorem 1.1')).toBeNull();
  });

  it('marks open prerequisites as assumptions and links the accepted version of the paper', async () => {
    renderWithApp(
      <ResultPage record="WP-2026-0001" claim="C1" />,
      fakeApi({
        getPaper: vi.fn(async () => ({
          ...publicPaper,
          publication: { version: 1, claims_revision: 2 },
          claims: claims.map((claim) => ({
            ...claim,
            ...(claim.id === 'C2' ? { kind: 'conjecture' as const, has_proof: false } : {}),
            lean: null,
          })),
        })),
      }),
    );
    expect(
      await screen.findByRole('heading', { name: 'Conditional on these assumptions' }),
    ).toBeVisible();
    expect(screen.queryByRole('heading', { name: 'Uses these results' })).toBeNull();
    expect(screen.getByRole('link', { name: 'Read the proof in the paper (PDF)' })).toHaveAttribute(
      'href',
      '/api/v1/submissions/sub-acc/files/pdf?version=1',
    );
  });

  it('does not describe an unproved conjecture as a proof', async () => {
    renderWithApp(
      <ResultPage record="WP-2026-0001" claim="C2" />,
      fakeApi({
        getPaper: vi.fn(async () => ({
          ...publicPaper,
          claims: claims.map((claim) => ({
            ...claim,
            ...(claim.id === 'C2' ? { kind: 'conjecture' as const, has_proof: false } : {}),
            lean: null,
          })),
        })),
      }),
    );
    expect(
      await screen.findByRole('link', { name: 'Read the statement in the paper (PDF)' }),
    ).toBeVisible();
    expect(screen.queryByRole('link', { name: 'Read the proof in the paper (PDF)' })).toBeNull();
    expect(screen.getByRole('heading', { name: 'Assumed in these statements' })).toBeVisible();
  });
});
