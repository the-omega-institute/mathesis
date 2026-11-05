import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import {
  dependencyKnowledge,
  dependencyPaper,
  dependencySource,
  dependencyUse,
} from '../test/dependencyFixtures';
import { loadDependencyGraph } from '../lib/dependencyGraph';
import { DependencyGraph, DependencyGraphView } from './DependencyGraph';
import { setLocale } from '../i18n/locale';

const makeApi = () =>
  fakeApi({
    researchGraph: vi.fn(async () => dependencySource),
    getCollectedPaper: async () => dependencyPaper,
    collectedKnowledge: async () => dependencyKnowledge,
  });
it('starts with a real graph and follows prerequisite and subsequent results with exact author credit', async () => {
  const api = makeApi();
  renderWithApp(<DependencyGraph />, api);
  const map = await screen.findByRole('figure', { name: 'Mathematical dependency graph' });
  expect(map.querySelectorAll('[data-relation="manuscript_reference"]')).toHaveLength(2);
  const detail = within(
    screen.getByRole('region', { name: 'Selected result and its dependencies' }),
  );
  expect(detail.getByRole('heading', { name: 'The uniform bound' })).toBeVisible();
  expect(detail.getByText('Original Author')).toBeVisible();
  expect(detail.queryByText('Lean checked')).toBeNull();
  expect(api.resultUses).not.toHaveBeenCalled();
  await userEvent.click(within(map).getByRole('button', { name: /Lemma: The local bound/ }));
  expect(detail.getByRole('heading', { name: 'The local bound' })).toBeVisible();
  expect(detail.getByRole('link', { name: 'Bounds on gaps' })).toHaveAttribute(
    'href',
    '/papers/gap-note?version=2',
  );
  await userEvent.click(
    detail.getByRole('button', { name: 'View this paper’s sources and results' }),
  );
  expect(within(map).getByRole('button', { name: /Earlier mathematics/ })).toBeVisible();
  await userEvent.click(within(map).getByRole('button', { name: /Earlier mathematics/ }));
  expect(detail.getByText('Earlier Author')).toBeVisible();
  expect(detail.getByRole('link', { name: 'Original source' })).toHaveAttribute(
    'href',
    'https://doi.org/10.1234/old',
  );
});
it('loads another page after rechecking current public visibility, without retaining removed old statements', async () => {
  const api = makeApi();
  let expanded = false;
  api.researchGraph = vi.fn(async ({ before } = {}) =>
    before
      ? { ...dependencySource, next_before: null }
      : expanded
        ? { ...dependencySource, nodes: [], edges: [], next_before: 'next' }
        : { ...dependencySource, next_before: 'next' },
  );
  renderWithApp(<DependencyGraph />, api);
  await screen.findByRole('figure');
  expanded = true;
  await userEvent.click(screen.getByRole('button', { name: 'Load more research into the graph' }));
  await screen.findByRole('figure');
  expect(api.researchGraph).toHaveBeenCalledTimes(3);
  expect(screen.getByText(/3 results loaded/)).toBeVisible();
});
it('keeps a selected checked intermediate proposition visible with its contributor and exact source result', async () => {
  const api = makeApi();
  const graph = await loadDependencyGraph(api, undefined, new AbortController().signal);
  const target = graph.nodes.find((n) => n.id === 'statement:gap-note:2:s2')!;
  target.collected = false;
  target.verified = true;
  api.resultUses = vi.fn(async () => [dependencyUse]);
  renderWithApp(<DependencyGraphView graph={graph} />, api);
  const map = screen.getByRole('figure');
  const helper = await within(map).findByRole('button', { name: /A checked intermediate bound/ });
  expect(map.querySelectorAll('[data-relation="checked_use"]')).toHaveLength(1);
  await userEvent.click(helper);
  const details = screen.getByRole('region', { name: 'Selected result and its dependencies' });
  expect(
    within(details).getByRole('heading', { name: 'A checked intermediate bound' }),
  ).toBeVisible();
  expect(details).toHaveTextContent('Earlier mathematician');
  expect(details).toHaveTextContent('Formalizer');
  expect(within(details).getByRole('link', { name: 'Read the paper' })).toHaveAttribute(
    'href',
    '/papers/WP-2026-0002/results/C1',
  );
});
it('keeps the graph as the primary view in Chinese and treats the theory outline as an auxiliary link', async () => {
  setLocale('zh');
  renderWithApp(<DependencyGraph />, makeApi());
  expect(await screen.findByRole('figure', { name: '数学依赖图' })).toBeVisible();
  expect(screen.getByRole('link', { name: '浏览已有理论' })).toHaveAttribute(
    'href',
    '/research-graph?focus=library%3Aall',
  );
  expect(screen.getByRole('heading', { name: 'The uniform bound' })).toBeVisible();
});
