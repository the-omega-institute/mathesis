import { describe, expect, it } from 'vitest';
import { fakeApi } from '../test/render';
import { publicPaper } from '../test/fixtures';
import {
  dependencyKnowledge,
  dependencyPaper,
  dependencySource,
  dependencyUse,
} from '../test/dependencyFixtures';
import {
  addCheckedUses,
  cleanSnapshot,
  dependencyLayout,
  dependencyAtlasLayout,
  loadDependencyGraph,
} from './dependencyGraph';
const signal = () => new AbortController().signal;
const collectedApi = () =>
  fakeApi({
    researchGraph: async () => dependencySource,
    getCollectedPaper: async () => dependencyPaper,
    collectedKnowledge: async () => dependencyKnowledge,
  });

describe('public dependency projection', () => {
  it('preserves exact statements, authors and versions; citations and membership never become proof dependencies', async () => {
    const graph = await loadDependencyGraph(collectedApi(), undefined, signal());
    expect(graph.nodes.find((n) => n.id === 'statement:gap-note:2:s2')).toMatchObject({
      statement: dependencyKnowledge.statements[1]!.statement,
      authors: ['Original Author'],
      paper: 'paper:gap-note:2',
    });
    expect(graph.edges).toContainEqual({
      from: 'statement:gap-note:2:s1',
      to: 'statement:gap-note:2:s2',
      relation: 'manuscript_reference',
    });
    expect(graph.edges).toContainEqual({
      from: 'doi:10.1234/old',
      to: 'paper:gap-note:2',
      relation: 'cites',
    });
    expect(graph.edges.filter((e) => e.relation === 'contains')).toHaveLength(3);
    expect(
      graph.edges.some((e) => e.relation === 'checked_use' || e.relation === 'declared_dependency'),
    ).toBe(false);
    expect(graph.nodes.some((n) => n.verified)).toBe(false);
    expect(graph.nodes.find((n) => n.kind === 'paper')!.leanSource).toBe(dependencyPaper.lean!.url);
  });
  it('keeps author-stated dependencies separate and never exposes claims from absent or changed publication snapshots', async () => {
    const source = {
      ...dependencySource,
      nodes: [
        {
          ...dependencySource.nodes[0]!,
          id: 'paper:WP-2026-0001',
          record: 'WP-2026-0001',
          version: 1,
        },
      ],
      edges: [],
    };
    const claims = [{ ...publicPaper.claims![0]!, depends_on: ['C3'] }, publicPaper.claims![1]!];
    const graph = await loadDependencyGraph(
      fakeApi({
        researchGraph: async () => source,
        getPaper: async () => ({
          ...publicPaper,
          publication: { version: 1, claims_revision: 1 },
          claims,
        }),
      }),
      undefined,
      signal(),
    );
    expect(graph.edges).toContainEqual({
      from: 'statement:WP-2026-0001:1:C3',
      to: 'statement:WP-2026-0001:1:C1',
      relation: 'declared_dependency',
    });
    for (const publication of [undefined, { version: 2, claims_revision: 1 }]) {
      const hidden = await loadDependencyGraph(
        fakeApi({
          researchGraph: async () => source,
          getPaper: async () => ({ ...publicPaper, publication }),
        }),
        undefined,
        signal(),
      );
      expect(hidden.nodes.filter((n) => n.kind === 'result')).toEqual([]);
      expect(hidden.edges).toEqual([]);
    }
  });
  it('uses exact proof-term evidence and keeps formalizer credit separate from original mathematical authorship', async () => {
    const source = await loadDependencyGraph(collectedApi(), undefined, signal());
    const graph = addCheckedUses(source, [
      dependencyUse,
      dependencyUse,
      { ...dependencyUse, result: { ...dependencyUse.result, version: 1 } },
    ]);
    expect(graph.nodes.filter((n) => n.kind === 'contribution')).toHaveLength(1);
    expect(graph.nodes.find((n) => n.kind === 'contribution')).toMatchObject({
      id: 'contribution:helper-proof:Research.bound',
      authors: ['Earlier mathematician'],
      contributor: { name: 'Formalizer' },
      verified: true,
    });
    expect(graph.edges.filter((e) => e.relation === 'checked_use')).toEqual([
      {
        from: 'contribution:helper-proof:Research.bound',
        to: 'statement:gap-note:2:s2',
        relation: 'checked_use',
      },
    ]);
    expect(graph.nodes.find((n) => n.id === 'statement:gap-note:2:s2')!.verified).toBe(false);
  });
  it('merges shared literature while keeping different source versions separate and discarding dangling edges', async () => {
    const first = await loadDependencyGraph(collectedApi(), undefined, signal());
    const second = await loadDependencyGraph(
      fakeApi({
        researchGraph: async () => ({
          ...dependencySource,
          nodes: dependencySource.nodes.map((n) =>
            n.kind === 'manuscript' ? { ...n, version: 3 } : n,
          ),
        }),
        getCollectedPaper: async () => ({ ...dependencyPaper, revision: '3' }),
        collectedKnowledge: async () => ({ ...dependencyKnowledge, version: 3 }),
      }),
      undefined,
      signal(),
    );
    const graph = cleanSnapshot({
      nodes: [...first.nodes, ...second.nodes],
      edges: [
        ...first.edges,
        ...first.edges,
        ...second.edges,
        { from: 'absent', to: first.nodes[0]!.id, relation: 'checked_use' },
      ],
      next: null,
    });
    expect(graph.nodes.filter((n) => n.kind === 'literature')).toHaveLength(1);
    expect(graph.nodes.filter((n) => n.kind === 'paper')).toHaveLength(2);
    expect(graph.edges.some((e) => e.from === 'absent')).toBe(false);
    expect(graph.nodes.find((n) => n.id === 'statement:gap-note:2:s2')!.paper).toBe(
      'paper:gap-note:2',
    );
    expect(graph.nodes.find((n) => n.id === 'statement:gap-note:3:s2')!.paper).toBe(
      'paper:gap-note:3',
    );
  });
  it('lays out cycles and large ranks deterministically without collapsing the map into a single tall column', async () => {
    const graph = await loadDependencyGraph(collectedApi(), undefined, signal());
    graph.edges.push({
      from: 'statement:gap-note:2:s2',
      to: 'statement:gap-note:2:s1',
      relation: 'manuscript_reference',
    });
    graph.nodes.push(
      ...Array.from({ length: 100 }, (_, i) => ({ ...graph.nodes[0]!, id: `isolated:${i}` })),
    );
    const layout = dependencyLayout(graph);
    expect(
      [...layout.points.values()].every((p) => Number.isFinite(p.x) && Number.isFinite(p.y)),
    ).toBe(true);
    expect(layout.height).toBeLessThan(1500);
    expect(dependencyLayout(graph)).toEqual(layout);
  });
});

it('keeps every exact result visible once in the field overview and adds no subject dependency edges', async () => {
  const graph = await loadDependencyGraph(collectedApi(), undefined, signal());
  const base = graph.nodes.find((node) => node.kind === 'result')!;
  graph.nodes.push({ ...base, id: 'other-result', areas: ['algebra', 'number-theory'] });
  const layout = dependencyAtlasLayout(graph, 1050);
  expect(layout.points.size).toBe(4);
  expect(layout.fields.map((field) => field.id)).toEqual(['algebra', 'number-theory']);
  expect(layout.fields.reduce((count, field) => count + field.count, 0)).toBe(4);
  expect(graph.edges.some((edge) => edge.relation === 'declared_dependency')).toBe(false);
  const mobile = dependencyAtlasLayout(graph, 350);
  expect([...mobile.points.values()].every((point) => point.x >= 0 && point.x < 350)).toBe(true);
});

it('projects collected verification only from the version-bound proof service, never from a source link', async () => {
  const graph = await loadDependencyGraph(
    fakeApi({
      researchGraph: async () => dependencySource,
      getCollectedPaper: async () => dependencyPaper,
      collectedKnowledge: async () => ({
        ...dependencyKnowledge,
        formalizations: [
          {
            claim: 's2',
            version: 2,
            state: 'verified',
            target_digest: 'exact-target',
            reading: 'Full statement.',
            next_step: 'Read proof.',
            proofs: [
              {
                id: 'checked-proof',
                entrant: dependencyUse.contributor,
                receipt: {
                  verdict: 'proved',
                  reason: 'Kernel checked',
                  target_digest: 'exact-target',
                  solution_digest: 'solution',
                  toolchain: 'pinned',
                  axioms: [],
                  checked_at: '2026-11-10T00:00:00Z',
                  duration: 1,
                },
                solution: 'checked source',
                also_verified: false,
              },
            ],
          },
          {
            claim: 's1',
            version: 2,
            state: 'open',
            target_digest: 'open-target',
            next_step: 'Contribute.',
            proofs: [],
          },
        ],
      }),
    }),
    undefined,
    signal(),
  );
  expect(graph.nodes.find((n) => n.claim === 's2')).toMatchObject({ verified: true, target: true });
  expect(graph.nodes.find((n) => n.claim === 's1')).toMatchObject({
    verified: false,
    target: true,
  });
  expect(graph.nodes.find((n) => n.claim === 's3')).toMatchObject({
    verified: false,
    target: false,
  });
});
