import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { ResearchGraph } from '../api/types';
import { fakeApi, renderWithApp } from '../test/render';
import { publicPaper } from '../test/fixtures';
import { PaperView } from './Papers';
import { GraphView, ResearchGraphPage } from './ResearchGraph';
import citationFixtures from '../test/research-citations.json';
import { parseRoute, routePath } from '../routing/routes';
const graph: ResearchGraph = {
  nodes: [
    {
      id: 'paper:WP-2026-0001',
      kind: 'manuscript',
      title: 'First manuscript',
      authors: ['A. New'],
      record: 'WP-2026-0001',
      url: null,
    },
    {
      id: 'paper:WP-2026-0002',
      kind: 'manuscript',
      title: 'Second manuscript',
      authors: ['B. New'],
      record: 'WP-2026-0002',
      url: null,
    },
    {
      id: 'doi:10.1234/old',
      kind: 'literature',
      title: 'An old result',
      authors: ['E. Earlier'],
      record: null,
      url: 'https://doi.org/10.1234/old',
    },
  ],
  edges: [1, 2].map((n) => ({
    from: `paper:WP-2026-000${n}`,
    to: 'doi:10.1234/old',
    relation: 'cites',
    record: `WP-2026-000${n}`,
    version: 1,
    reference_key: 'old',
    source_location: 'refs.bib',
    title: 'An old result',
    authors: ['E. Earlier'],
    year: '1901',
    raw: 'E. Earlier, An old result, 1901.',
    raw_truncated: false,
  })),
  coverage: [
    {
      record: 'WP-2026-0001',
      version: 1,
      method: 'latex_literal_citations_v1',
      limitations: ['Bounded extraction'],
    },
  ],
  next_before: 'WP-2026-0001',
  scope: 'Source-declared citations. Coverage is incomplete.',
};
describe('shared research graph', () => {
  it('shows an earlier work as an entry with two citing manuscripts and source attribution', () => {
    renderWithApp(<GraphView graph={graph} focus="doi:10.1234/old" />, fakeApi());
    expect(screen.getByRole('heading', { name: 'Cited by' })).toBeVisible();
    expect(screen.getByRole('link', { name: 'Original source' })).toHaveAttribute(
      'href',
      'https://doi.org/10.1234/old',
    );
    expect(
      screen
        .getAllByRole('link', { name: 'First manuscript' })
        .find((link) => link.getAttribute('href') === '/papers/WP-2026-0001'),
    ).toHaveAttribute('href', '/papers/WP-2026-0001');
    expect(
      screen
        .getAllByRole('link', { name: 'Second manuscript' })
        .find((link) => link.getAttribute('href') === '/papers/WP-2026-0002'),
    ).toHaveAttribute('href', '/papers/WP-2026-0002');
    expect(screen.getAllByText(/E\. Earlier/).length).toBeGreaterThanOrEqual(3);
    expect(screen.getByRole('link', { name: 'Next page of matching works' })).toHaveAttribute(
      'href',
      '/research-graph?focus=doi%3A10.1234%2Fold&before=WP-2026-0001',
    );
  });
  it('uses relationships as the global view without repeating a connected-work list', () => {
    renderWithApp(<GraphView graph={graph} />, fakeApi());
    expect(screen.getByRole('heading', { name: 'Relationships in this view' })).toBeVisible();
    expect(screen.queryByRole('heading', { name: 'Works in this view' })).toBeNull();
    expect(screen.queryByText(/No matching public work/)).toBeNull();
  });
  it('preserves scoped query identities containing slashes across route round trips', () => {
    const route = {
      kind: 'research_graph' as const,
      focus: 'arxiv:math/0301234v2',
      before: 'WP-2026-0002',
    };
    const url = new URL(routePath(route), 'https://example.test');
    expect(parseRoute(url.pathname, url.search)).toEqual(route);
  });
  it('requests a bounded page without inferring exhaustive results from an empty match', async () => {
    const api = fakeApi({ researchGraph: vi.fn(async () => ({ ...graph, nodes: [], edges: [] })) });
    renderWithApp(<ResearchGraphPage q="Earlier" />, api);
    expect(await screen.findByText(/No matching public work was found/)).toBeVisible();
    expect(api.researchGraph).toHaveBeenCalledWith(
      { q: 'Earlier', focus: undefined, before: undefined },
      expect.objectContaining({ signal: expect.any(AbortSignal) }),
    );
    expect(screen.getByRole('link', { name: 'Next page of matching works' })).toBeVisible();
  });
  it('keeps source evidence readable and unsafe supplied links inert on a paper', async () => {
    const user = userEvent.setup();
    renderWithApp(
      <PaperView
        paper={{
          ...publicPaper,
          publication: { version: 1, claims_revision: 1 },
          bibliography: {
            method: 'latex_literal_citations_v1',
            limitations: ['Incomplete coverage'],
            references: [
              {
                work: 'doi:10.1234/old',
                reference: {
                  key: 'old',
                  title: 'An old result',
                  authors: ['E. Earlier'],
                  year: '1901',
                  doi: '10.1234/old',
                  arxiv: null,
                  url: 'javascript:alert(1)',
                  source_location: 'refs.bib',
                  raw: 'E. Earlier, original reference.',
                  raw_truncated: false,
                },
              },
            ],
          },
        }}
      />,
      fakeApi(),
    );
    expect(screen.getByRole('link', { name: 'An old result' })).toHaveAttribute(
      'href',
      '/research-graph?focus=doi%3A10.1234%2Fold',
    );
    expect(screen.queryByRole('link', { name: 'Source link' })).toBeNull();
    await user.click(screen.getByText('Source evidence'));
    expect(screen.getByText('E. Earlier, original reference.')).toBeVisible();
    expect(screen.getByText(/They do not establish a checked/)).toBeVisible();
  });
  for (const fixture of citationFixtures) {
    it(`uses actual-parser ${fixture.kind} prose across paper, relationships and focused work`, async () => {
      const user = userEvent.setup();
      const occurrence = fixture.expected;
      const reference = occurrence.reference;
      const paper = {
        ...publicPaper,
        publication: { version: 1, claims_revision: 1 },
        bibliography: {
          method: 'latex_literal_citations_v1',
          limitations: ['Literal source coverage'],
          references: [occurrence],
        },
      };
      const { unmount } = renderWithApp(<PaperView paper={paper} />, fakeApi());
      expect(screen.getByRole('link', { name: occurrence.display_text })).toBeVisible();
      expect(
        screen.queryByRole('link', { name: /internalKey17|\\bibitem|\\doi|\\href/ }),
      ).toBeNull();
      expect(screen.getByText(reference.raw)).not.toBeVisible();
      await user.click(screen.getByText('Source evidence'));
      expect(screen.getByText(reference.raw)).toBeVisible();
      unmount();
      const inlineGraph: ResearchGraph = {
        ...graph,
        nodes: [
          graph.nodes[0]!,
          { ...graph.nodes[2]!, title: occurrence.display_text, authors: [] },
        ],
        edges: [{ ...graph.edges[0]!, ...reference, display_text: occurrence.display_text }],
        next_before: null,
      };
      renderWithApp(<GraphView graph={inlineGraph} focus={occurrence.work} />, fakeApi());
      expect(screen.getByRole('heading', { name: 'Selected earlier work' })).toBeVisible();
      expect(screen.getAllByRole('link', { name: occurrence.display_text })).toHaveLength(2);
      expect(
        screen.queryByRole('link', { name: /internalKey17|\\bibitem|\\doi|\\href/ }),
      ).toBeNull();
      expect(screen.getByText(reference.raw)).not.toBeVisible();
      await user.click(screen.getByText('Source evidence'));
      expect(screen.getByText(reference.raw)).toBeVisible();
    });
  }
  it('submits an ordinary identifier search and preserves the matching-result cursor', async () => {
    const user = userEvent.setup();
    const api = fakeApi({ researchGraph: vi.fn(async () => graph) });
    renderWithApp(<ResearchGraphPage q="10.1234/old" before="WP-2026-0002" />, api);
    await screen.findByRole('heading', { name: 'Relationships in this view' });
    expect(api.researchGraph).toHaveBeenCalledWith(
      { q: '10.1234/old', before: 'WP-2026-0002', focus: undefined },
      expect.objectContaining({ signal: expect.any(AbortSignal) }),
    );
    expect(screen.getByRole('link', { name: 'Next page of matching works' })).toHaveAttribute(
      'href',
      '/research-graph?q=10.1234%2Fold&before=WP-2026-0001',
    );
    await user.clear(screen.getByLabelText('Find a work, identifier or cited author'));
    await user.type(screen.getByLabelText('Find a work, identifier or cited author'), 'E. Earlier');
    await user.click(screen.getByRole('button', { name: 'Search graph' }));
    expect(window.location.search).toBe('?q=E.+Earlier');
  });
});
