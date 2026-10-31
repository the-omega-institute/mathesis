import { screen, within } from '@testing-library/react';
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
  it('opens directly into a graph with source credit and no search form', async () => {
    renderWithApp(<ResearchGraphPage />, fakeApi({ researchGraph: async () => graph }));
    const map = await screen.findByRole('figure', { name: 'Research connections' });
    expect(within(map).getByRole('button', { name: /An old result E\. Earlier/ })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(within(map).getByRole('button', { name: /First manuscript A\. New/ })).toBeVisible();
    expect(within(map).getByRole('button', { name: /Second manuscript B\. New/ })).toBeVisible();
    expect(map.querySelectorAll('.research-map-edge')).toHaveLength(2);
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Search graph' })).toBeNull();
    expect(
      within(screen.getByRole('region', { name: 'Selected work' })).getByText('E. Earlier'),
    ).toBeVisible();
  });
  it('selects a paper to show its authors and follows the connection back to prior work', async () => {
    const user = userEvent.setup();
    renderWithApp(<GraphView graph={graph} />, fakeApi());
    const map = screen.getByRole('figure', { name: 'Research connections' });
    await user.click(within(map).getByRole('button', { name: /First manuscript A\. New/ }));
    let detail = within(screen.getByRole('region', { name: 'Selected work' }));
    expect(detail.getByRole('heading', { name: 'First manuscript' })).toBeVisible();
    expect(detail.getByText('A. New')).toBeVisible();
    expect(detail.getByRole('link', { name: 'Read paper and results' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001',
    );
    expect(detail.getByRole('link', { name: 'Explore these connections' })).toHaveAttribute(
      'href',
      '/research-graph?focus=paper%3AWP-2026-0001',
    );
    await user.click(detail.getByRole('button', { name: 'An old result' }));
    detail = within(screen.getByRole('region', { name: 'Selected work' }));
    expect(detail.getByRole('heading', { name: 'Cited by' })).toBeVisible();
    expect(detail.getByRole('link', { name: 'Original source' })).toHaveAttribute(
      'href',
      'https://doi.org/10.1234/old',
    );
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
    expect(await screen.findByText(/No public work is available for this view/)).toBeVisible();
    expect(api.researchGraph).toHaveBeenCalledWith(
      { q: 'Earlier', focus: undefined, before: undefined },
      expect.objectContaining({ signal: expect.any(AbortSignal) }),
    );
    expect(screen.getByRole('link', { name: 'Next group of papers' })).toBeVisible();
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
      expect(screen.getByRole('heading', { name: occurrence.display_text })).toBeVisible();
      expect(
        within(screen.getByRole('figure')).getByRole('button', { name: occurrence.display_text }),
      ).toBeVisible();
      expect(
        screen.queryByRole('link', { name: /internalKey17|\\bibitem|\\doi|\\href/ }),
      ).toBeNull();
      expect(screen.getByText(reference.raw)).not.toBeVisible();
      await user.click(screen.getByText('Source evidence'));
      expect(screen.getByText(reference.raw)).toBeVisible();
    });
  }
  it('preserves existing query URLs without presenting a separate search form', async () => {
    const api = fakeApi({ researchGraph: vi.fn(async () => graph) });
    renderWithApp(<ResearchGraphPage q="10.1234/old" before="WP-2026-0002" />, api);
    await screen.findByRole('figure', { name: 'Research connections' });
    expect(api.researchGraph).toHaveBeenCalledWith(
      { q: '10.1234/old', before: 'WP-2026-0002', focus: undefined },
      expect.objectContaining({ signal: expect.any(AbortSignal) }),
    );
    expect(screen.getByRole('link', { name: 'Next group of papers' })).toHaveAttribute(
      'href',
      '/research-graph?q=10.1234%2Fold&before=WP-2026-0001',
    );
    expect(screen.queryByRole('textbox')).toBeNull();
  });
  it('keeps larger reference sets reachable without crowding the map', async () => {
    const user = userEvent.setup();
    const refs = Array.from({ length: 17 }, (_, n) => ({
      ...graph.nodes[2]!,
      id: `doi:10.1234/ref${n}`,
      title: `Reference ${String(n).padStart(2, '0')}`,
    }));
    const many: ResearchGraph = {
      ...graph,
      nodes: [graph.nodes[0]!, ...refs],
      edges: refs.map((ref, i) => ({
        ...graph.edges[0]!,
        to: ref.id,
        reference_key: `r${i}`,
        title: ref.title,
      })),
      next_before: null,
    };
    renderWithApp(<GraphView graph={many} focus={graph.nodes[0]!.id} />, fakeApi());
    const map = screen.getByRole('figure', { name: 'Research connections' });
    expect(within(map).getAllByRole('button', { name: /^Reference/ })).toHaveLength(8);
    await user.click(within(map).getByRole('button', { name: 'Next references' }));
    expect(within(map).getByText('9–16 of 17 references in this view')).toBeVisible();
    await user.click(within(map).getByRole('button', { name: /First manuscript A\. New/ }));
    await user.click(
      within(screen.getByRole('region', { name: 'Selected work' })).getByRole('button', {
        name: 'Reference 16',
      }),
    );
    expect(within(map).getByText('17–17 of 17 references in this view')).toBeVisible();
    expect(within(map).getByRole('button', { name: /Reference 16 E\. Earlier/ })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(within(map).getByRole('button', { name: 'Next references' })).toBeDisabled();
  });
  it('clears selected credit when refreshed public data removes a work', () => {
    const { rerender } = renderWithApp(<GraphView graph={graph} />, fakeApi());
    expect(screen.getByRole('heading', { name: 'An old result' })).toBeVisible();
    rerender(
      <GraphView graph={{ ...graph, nodes: [], edges: [], coverage: [], next_before: null }} />,
    );
    expect(screen.queryByText('E. Earlier')).toBeNull();
    expect(screen.queryByRole('region', { name: 'Selected work' })).toBeNull();
    expect(screen.queryByRole('figure')).toBeNull();
    expect(screen.getByRole('heading', { name: 'No public connections yet' })).toBeVisible();
  });
  it('draws repeated citations once and retains all source observations', async () => {
    const user = userEvent.setup();
    renderWithApp(
      <GraphView
        graph={{
          ...graph,
          edges: [
            ...graph.edges,
            { ...graph.edges[0]!, reference_key: 'other', raw: 'A second source observation.' },
          ],
        }}
      />,
      fakeApi(),
    );
    expect(screen.getByRole('figure').querySelectorAll('.research-map-edge')).toHaveLength(2);
    expect(screen.getByText('A second source observation.')).not.toBeVisible();
    await user.click(screen.getByText('Source evidence'));
    expect(screen.getByText('A second source observation.')).toBeVisible();
  });
  it('displays an isolated paper without inventing a citation', () => {
    renderWithApp(
      <GraphView graph={{ ...graph, nodes: [graph.nodes[0]!], edges: [], next_before: null }} />,
      fakeApi(),
    );
    expect(screen.getByRole('figure').querySelectorAll('.research-map-edge')).toHaveLength(0);
    expect(screen.getByRole('heading', { name: 'First manuscript' })).toBeVisible();
    expect(screen.getByText(/No citation connections are available/)).toBeVisible();
  });
});
