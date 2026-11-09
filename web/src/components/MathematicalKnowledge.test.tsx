import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import {
  MathematicalKnowledge,
  ResultConstellation,
  type KnowledgeReading,
} from './MathematicalKnowledge';
import { publicPaper } from '../test/fixtures';
import type { CollectedPaper } from '../api/types';
const reading: KnowledgeReading = {
  work: 'gap-note',
  version: '2',
  title: 'Bounds on gaps',
  authors: [{ name: 'Original Author' }],
  collected: true,
  macros: {},
  lean: 'https://github.com/example/project/blob/0123456789012345678901234567890123456789/Gap.lean',
  limitations: [],
  statements: [
    {
      id: 's1',
      kind: 'lemma',
      name: 'Lemma',
      title: 'The local bound',
      statement: 'Every gap is at most $2$.',
    },
    {
      id: 's2',
      kind: 'theorem',
      name: 'Theorem',
      title: 'The uniform bound',
      statement: 'For every $n$, the gap is at most $2$.',
    },
    { id: 's3', kind: 'corollary', name: 'Corollary', statement: 'The sequence has bounded gaps.' },
  ],
  connections: [{ from: 's2', to: 's1' }],
};
const paper: CollectedPaper = {
  id: reading.work,
  revision: '2',
  title: reading.title,
  abstract: 'A bound.',
  authors: reading.authors,
  license: 'CC-BY-4.0',
  msc: [],
  problems: [],
  published_on: '2026-11-05T10:00:00Z',
  lean: { url: reading.lean!, revision: '0123456789012345678901234567890123456789' },
  pdf: { url: '/pdf' },
  source: { url: '/source' },
};
describe('mathematical knowledge', () => {
  it('makes exact results the nodes and follows source references without awarding proof credit', async () => {
    const user = userEvent.setup();
    renderWithApp(<ResultConstellation reading={reading} />, fakeApi());
    const map = screen.getByRole('figure', { name: 'Mathematical results and connections' });
    expect(
      await within(map).findByRole('button', { name: /Theorem: The uniform bound/ }),
    ).toHaveAttribute('aria-pressed', 'true');
    expect(map.querySelectorAll('[data-relation="manuscript_reference"]')).toHaveLength(1);
    const detail = within(screen.getByRole('region', { name: 'Selected mathematical statement' }));
    expect(detail.getByRole('heading', { name: 'Theorem: The uniform bound' })).toBeVisible();
    expect(detail.getByText('Original Author')).toBeVisible();
    expect(
      detail.getByText('A confirmed Lean target is not yet available for this statement.'),
    ).toBeVisible();
    expect(
      detail.getByRole('link', { name: 'Lean source accompanying the paper' }),
    ).toHaveAttribute('href', reading.lean);
    expect(screen.queryByText('Lean verified')).toBeNull();
    expect(screen.queryByRole('link', { name: 'Contribute a proof' })).toBeNull();
    expect(screen.getByRole('link', { name: 'Link to this result' })).toHaveAttribute(
      'href',
      '/formalizations?focus=statement%3Agap-note%3A2%3As2',
    );
    await user.click(within(map).getByRole('button', { name: /Lemma: The local bound/ }));
    expect(detail.getByRole('heading', { name: 'Lemma: The local bound' })).toBeVisible();
    expect(detail.getByText('Referenced by')).toBeVisible();
  });
  it('shows a contribution entry only for a confirmed target and no invented edges for isolated results', async () => {
    renderWithApp(
      <ResultConstellation
        reading={{
          ...reading,
          collected: false,
          connections: [],
          statements: [{ ...reading.statements[1]!, target: true }],
        }}
      />,
      fakeApi(),
    );
    expect(screen.getByRole('figure').querySelectorAll('.knowledge-result-edge')).toHaveLength(0);
    expect(await screen.findByRole('link', { name: 'Contribute a proof' })).toHaveAttribute(
      'href',
      '/submit?type=lean&record=gap-note&claim=s2',
    );
  });
  it('opens an exact old source version even when a newer paper is on the current page', async () => {
    const api = fakeApi({
      listCollectedPapers: async () => ({ items: [paper], next_before: null }),
      getCollectedPaper: vi.fn(async () => ({ ...paper, revision: '1' })),
      collectedKnowledge: vi.fn(async () => ({
        work: 'gap-note',
        version: 1,
        statements: reading.statements,
        connections: reading.connections.map((e) => ({
          ...e,
          relation: 'references_in_manuscript' as const,
        })),
        macros: {},
        limitations: [],
      })),
    });
    renderWithApp(<MathematicalKnowledge focus="statement:gap-note:1:s1" />, api);
    expect(await screen.findByRole('heading', { name: 'Lemma: The local bound' })).toBeVisible();
    expect(api.getCollectedPaper).toHaveBeenCalledWith('gap-note', '1', expect.anything());
    expect(api.collectedKnowledge).toHaveBeenCalledWith('gap-note', '1', expect.anything());
    expect(
      screen.getByRole('link', { name: 'Read the paper and plain-language explanation' }),
    ).toHaveAttribute('href', '/papers/gap-note?version=1');
  });
  it('continues through collected pages and then all ordinary paper pages', async () => {
    const user = userEvent.setup();
    const summary = { ...publicPaper.summary, accepted_at: '2026-11-01' };
    const api = fakeApi({
      listCollectedPapers: vi.fn(async ({ before } = {}) => ({
        items: [{ ...paper, id: before ? 'second-note' : 'gap-note' }],
        next_before: before ? null : 'collection-next',
      })),
      collectedKnowledge: async () => ({
        work: 'gap-note',
        version: 2,
        statements: reading.statements,
        connections: [],
        macros: {},
        limitations: [],
      }),
      listPapers: vi.fn(async ({ before } = {}) => ({
        items: [{ ...summary, record: before ? 'WP-2026-0002' : 'WP-2026-0001' }],
        next_before: before ? null : 'ordinary-next',
      })),
      getPaper: async () => publicPaper,
    });
    renderWithApp(<MathematicalKnowledge />, api);
    await screen.findByRole('heading', { name: 'Bounds on gaps' });
    await user.click(screen.getByRole('button', { name: 'More papers' }));
    await screen.findByRole('option', { name: 'Bounds on gaps' });
    expect(api.listCollectedPapers).toHaveBeenCalledWith(
      { limit: 12, before: 'collection-next' },
      expect.anything(),
    );
    await user.click(screen.getByRole('button', { name: 'More papers' }));
    await screen.findByRole('option', { name: summary.title });
    expect(api.listPapers).toHaveBeenCalledWith(
      { limit: 12, before: undefined },
      expect.anything(),
    );
    await user.click(screen.getByRole('button', { name: 'More papers' }));
    await screen.findByRole('option', { name: summary.title });
    expect(api.listPapers).toHaveBeenCalledWith(
      { limit: 12, before: 'ordinary-next' },
      expect.anything(),
    );
    expect(screen.queryByRole('button', { name: 'More papers' })).toBeNull();
  });
  it('keeps reciprocal manuscript references without duplicating result nodes', () => {
    renderWithApp(
      <ResultConstellation
        reading={{
          ...reading,
          connections: [
            { from: 's2', to: 's1' },
            { from: 's1', to: 's2' },
          ],
        }}
      />,
      fakeApi(),
    );
    const map = screen.getByRole('figure');
    expect(within(map).getAllByRole('button', { name: 'Lemma: The local bound' })).toHaveLength(1);
    expect(map.querySelectorAll('[data-relation="manuscript_reference"]')).toHaveLength(2);
  });
  it('ends collection navigation when there are no ordinary publications', async () => {
    const api = fakeApi({
      listCollectedPapers: async () => ({ items: [paper], next_before: null }),
      collectedKnowledge: async () => ({
        work: 'gap-note',
        version: 2,
        statements: reading.statements,
        connections: [],
        macros: {},
        limitations: [],
      }),
    });
    renderWithApp(<MathematicalKnowledge />, api);
    await screen.findByRole('heading', { name: 'Bounds on gaps' });
    expect(screen.queryByRole('button', { name: 'More papers' })).toBeNull();
  });
});
