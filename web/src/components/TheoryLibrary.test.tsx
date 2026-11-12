import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { paperSummary } from '../test/fixtures';
import type { CollectedPaper } from '../api/types';
import { formalLibrary, moduleSource } from '../lib/theoryLibrary';
import { LibraryBranch, TheoryLibrary } from './TheoryLibrary';

const paper: CollectedPaper = {
  id: 'kmvq31-sl3-obstruction',
  revision: '2',
  title: 'Special linear obstructions',
  abstract: 'A statement about evolution algebras.',
  authors: [{ name: 'Original Author' }],
  license: 'CC-BY-4.0',
  msc: [],
  problems: [],
  published_on: '2026-11-05',
  pdf: { url: '/pdf', bytes: 1, sha256: 'a'.repeat(64) },
  source: { url: '/source', bytes: 1, sha256: 'b'.repeat(64) },
};

describe('theory library reading', () => {
  it('starts from all fields and existing theory without choosing a paper or loading graph data', () => {
    const api = fakeApi();
    renderWithApp(<TheoryLibrary />, api);
    expect(screen.getByRole('link', { name: 'Algebra' })).toHaveAttribute(
      'href',
      '/formalizations?focus=library%3Aalgebra',
    );
    expect(screen.getByRole('link', { name: 'Topology' })).toBeVisible();
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(api.listCollectedPapers).not.toHaveBeenCalled();
    expect(api.researchGraph).not.toHaveBeenCalled();
  });
  it('reads established resources before research and keeps source authorship and proof status distinct', async () => {
    const api = fakeApi({
      listCollectedPapers: async () => ({ items: [paper], next_before: null }),
    });
    renderWithApp(<TheoryLibrary focus="theory:algebra" />, api);
    expect(screen.getByRole('heading', { name: 'Established theory' })).toBeVisible();
    expect(screen.getByRole('link', { name: 'Sylow theory' })).toHaveAttribute(
      'href',
      moduleSource('Mathlib.GroupTheory.Sylow'),
    );
    expect(
      await screen.findByRole('link', { name: 'Special linear obstructions' }),
    ).toHaveAttribute('href', '/papers/kmvq31-sl3-obstruction?version=2');
    expect(
      screen.getByRole('link', { name: 'Read the results and their connections' }),
    ).toHaveAttribute('href', '/formalizations?focus=statement%3Akmvq31-sl3-obstruction%3A2%3As1');
    expect(screen.getByText('Original Author')).toBeVisible();
    expect(screen.queryByText('Lean verified')).toBeNull();
    expect(screen.getByText(/Papers are placed by subject/)).toBeVisible();
    expect(api.collectedKnowledge).not.toHaveBeenCalled();
    await userEvent.click(screen.getByText(/Browse the full formal library/));
    expect(screen.getByText('Group Theory', { exact: true })).toBeVisible();
    expect(screen.getByRole('link', { name: 'Mathlib source and contributors' })).toHaveAttribute(
      'href',
      `${formalLibrary.repository}/tree/${formalLibrary.revision}/Mathlib`,
    );
  });
  it('keeps later source pages reachable and separates unrelated papers by topic', async () => {
    const api = fakeApi({
      listPapers: vi.fn(async () => ({ items: [], next_before: null })),
      listCollectedPapers: async ({ before } = {}) => ({
        items: before ? [paper] : [{ ...paper, id: 'unknown-paper', title: 'Unclassified paper' }],
        next_before: before ? null : 'page-two',
      }),
    });
    renderWithApp(<TheoryLibrary focus="theory:algebra" />, api);
    expect(await screen.findByText(/No papers from this field/)).toBeVisible();
    expect(screen.queryByRole('link', { name: 'Unclassified paper' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Next →' }));
    expect(await screen.findByRole('link', { name: 'Special linear obstructions' })).toBeVisible();
    expect(api.listPapers).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: '← Previous' }));
    expect(await screen.findByText(/No papers from this field/)).toBeVisible();
    expect(screen.queryByRole('link', { name: 'Special linear obstructions' })).toBeNull();
  });
  it('shows the first four papers and expands the remainder without a wall of entries', async () => {
    const api = fakeApi({
      listCollectedPapers: async () => ({
        items: Array.from({ length: 6 }, (_, n) => ({
          ...paper,
          id: 'p' + n,
          title: 'Paper ' + n,
          msc: ['15A01'],
        })),
        next_before: null,
      }),
    });
    renderWithApp(<TheoryLibrary focus="theory:linear-algebra" />, api);
    await screen.findByRole('link', { name: 'Paper 0' });
    expect(screen.queryByRole('link', { name: 'Paper 5' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Show all 6 papers on this page' }));
    expect(screen.getByRole('link', { name: 'Paper 5' })).toBeVisible();
  });
  it('reads both public paper services together and keeps their independent pagination', async () => {
    const api = fakeApi({
      listCollectedPapers: async ({ before } = {}) => ({
        items: [{ ...paper, id: before ? 'second-note' : paper.id, msc: ['20A05'] }],
        next_before: before ? null : 'notes-page-two',
      }),
      listPapers: vi.fn(async ({ before } = {}) => ({
        items: [
          {
            ...paperSummary,
            title: before ? 'Later accepted paper' : 'Accepted paper',
            msc: ['20A05'],
          },
          {
            ...paperSummary,
            record: 'not-accepted',
            title: 'Undecided paper',
            accepted_at: undefined,
            msc: ['20A05'],
          },
        ],
        next_before: before ? null : 'papers-page-two',
      })),
    });
    renderWithApp(<TheoryLibrary focus="theory:algebra" />, api);
    expect(await screen.findByRole('link', { name: 'Accepted paper' })).toHaveAttribute(
      'href',
      `/papers/${paperSummary.record}`,
    );
    expect(screen.getByRole('link', { name: 'Special linear obstructions' })).toBeVisible();
    expect(screen.queryByText('Undecided paper')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Next →' }));
    expect(await screen.findByRole('link', { name: 'Later accepted paper' })).toBeVisible();
    expect(api.listPapers).toHaveBeenLastCalledWith(
      { limit: 100, before: 'papers-page-two' },
      { signal: expect.any(AbortSignal) },
    );
    expect(screen.getByRole('link', { name: 'Special linear obstructions' })).toHaveAttribute(
      'href',
      '/papers/second-note?version=2',
    );
  });
  it('expands real source branches lazily and keeps the parent source file available', async () => {
    renderWithApp(
      <ul>
        <LibraryBranch
          prefix="Mathlib.Tactic.Ring"
          modules={['Mathlib.Tactic.Ring', 'Mathlib.Tactic.Ring.Basic']}
        />
      </ul>,
      fakeApi(),
    );
    expect(screen.queryByRole('link', { name: 'Basic' })).toBeNull();
    await userEvent.click(screen.getByText('Ring', { exact: true }));
    expect(screen.getByRole('link', { name: 'Read the definitions and results' })).toHaveAttribute(
      'href',
      moduleSource('Mathlib.Tactic.Ring'),
    );
    expect(screen.getByRole('link', { name: 'Basic' })).toHaveAttribute(
      'href',
      moduleSource('Mathlib.Tactic.Ring.Basic'),
    );
  });
});
