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
  renderWithApp(<DependencyGraph initialScope="all" />, api);
  const map = await screen.findByRole('figure', { name: 'Mathematical dependency graph' });
  expect(map.querySelectorAll('[data-relation="manuscript_reference"]')).toHaveLength(2);
  await userEvent.click(within(map).getByRole('button', { name: /Theorem: The uniform bound/ }));
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
  api.researchGraph = vi.fn(async ({ before } = {}) => {
    if (!before)
      return {
        ...dependencySource,
        nodes: expanded ? [] : dependencySource.nodes,
        edges: expanded ? [] : dependencySource.edges,
        next_before: 'two',
      };
    if (before === 'two') return { ...dependencySource, next_before: 'three' };
    return { ...dependencySource, next_before: null };
  });
  renderWithApp(<DependencyGraph initialScope="all" />, api);
  await screen.findByRole('figure');
  expanded = true;
  await userEvent.click(screen.getByRole('button', { name: 'Load more research into the graph' }));
  await screen.findByRole('figure');
  expect(api.researchGraph).toHaveBeenCalledTimes(5);
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
    '/papers/WP-2026-0002?version=1',
  );
});
it('keeps the graph as the primary view in Chinese and treats the theory outline as an auxiliary link', async () => {
  setLocale('zh');
  renderWithApp(<DependencyGraph initialScope="all" />, makeApi());
  expect(await screen.findByRole('figure', { name: '数学依赖图' })).toBeVisible();
  expect(screen.getByRole('link', { name: '浏览已有理论' })).toHaveAttribute(
    'href',
    '/formalizations?focus=library%3Aall',
  );
  expect(screen.queryByRole('region', { name: '所选结果及其依赖' })).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: /Theorem: The uniform bound/ }));
  expect(screen.getByRole('heading', { name: 'The uniform bound' })).toBeVisible();
});

it('shows results from every loaded field first and restores the whole overview after reading one result', async () => {
  const api = makeApi();
  const base = await loadDependencyGraph(api, undefined, new AbortController().signal);
  const earlier = base.nodes.find((node) => node.id === 'statement:gap-note:2:s1')!;
  const algebra = {
    ...earlier,
    id: 'statement:algebra-note:1:s1',
    title: 'An algebraic consequence',
    areas: ['algebra'],
  };
  const graph = {
    ...base,
    nodes: [...base.nodes, algebra],
    edges: [
      ...base.edges,
      { from: earlier.id, to: algebra.id, relation: 'declared_dependency' as const },
    ],
  };
  renderWithApp(<DependencyGraphView graph={graph} />, api);
  let map = screen.getByRole('figure');
  expect(within(map).getByRole('button', { name: /Algebra\s*1 results/ })).toBeVisible();
  expect(within(map).getByRole('button', { name: /Number theory\s*3 results/ })).toBeVisible();
  expect(map.querySelectorAll('.dependency-node')).toHaveLength(5);
  expect(map.querySelectorAll('[data-relation="declared_dependency"]')).toHaveLength(1);
  expect(screen.queryByRole('region', { name: 'Selected result and its dependencies' })).toBeNull();
  await userEvent.click(within(map).getByRole('button', { name: /An algebraic consequence/ }));
  expect(screen.getByRole('heading', { name: 'An algebraic consequence' })).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: 'All fields' }));
  map = screen.getByRole('figure');
  expect(map.querySelectorAll('.dependency-node')).toHaveLength(5);
  expect(within(map).getByRole('button', { name: /Number theory\s*3 results/ })).toBeVisible();
});

it('shows published sources without labelling every manuscript statement as missing a proof or awarding new proof credit', async () => {
  renderWithApp(<DependencyGraph />, makeApi());
  await screen.findByRole('figure');
  expect(screen.getByRole('button', { name: /^All results/ })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  const pending = screen.getByRole('button', { name: /Theorem: The uniform bound/ });
  expect(pending).toHaveAttribute('data-verification', 'source_available');
  await userEvent.click(pending);
  expect(pending).not.toHaveClass('is-verified');
  expect(screen.getByText(/Read its scope before proposing additional proof work/)).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: /^Published Lean sources/ }));
  const source = screen.getByRole('button', { name: /Published Lean source: Bounds on gaps/ });
  expect(source).toHaveAttribute('data-verification', 'source_available');
  expect(source).not.toHaveClass('is-verified');
  await userEvent.click(source);
  expect(
    screen.getByRole('link', { name: 'Published Lean source for this paper' }),
  ).toHaveAttribute('href', dependencyPaper.lean!.url);
  expect(screen.getByText(/Original attribution is retained in the source/)).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: /^Proof opportunities/ }));
  expect(screen.queryByRole('button', { name: /Theorem: The uniform bound/ })).toBeNull();
  expect(screen.getByText(/No confirmed proof gaps are listed/)).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: /^Checked statements/ }));
  expect(screen.queryByRole('button', { name: /Theorem: The uniform bound/ })).toBeNull();
  expect(screen.getByText(/No independently verified formalizations yet/)).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: /^All results/ }));
  expect(await screen.findByRole('button', { name: /Theorem: The uniform bound/ })).toBeVisible();
});
