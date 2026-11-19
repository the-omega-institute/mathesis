import { describe, expect, it } from 'vitest';
import type { CollectedPaper, CollectedKnowledge } from '../api/types';
import type { SharedLibrary } from './sharedLibrary';
import {
  formalizationLibrary,
  layoutFormalizationLibrary,
  subjectFormalizationLibrary,
} from './formalizationLibrary';

const commit = 'b'.repeat(40);
const library: SharedLibrary = {
  sourceCommit: commit,
  digest: 'sha256:' + 'a'.repeat(64),
  nodes: [
    {
      id: 'Quantum/Base',
      title: 'Projection bound',
      abstract: '',
      domain: 'Quantum',
      source: `https://github.com/the-omega-institute/trureturing/blob/${commit}/Quantum/Base.lean`,
      formalized: true,
      x: 1000,
      y: 2000,
    },
    {
      id: 'Arith/Base',
      title: 'Integer identity',
      abstract: '',
      domain: 'Arith',
      source: '',
      formalized: true,
      x: 3000,
      y: 4000,
    },
  ],
  edges: [{ from: 'Arith/Base', to: 'Quantum/Base' }],
};
const paper = {
  id: 'paper-one',
  revision: '2',
  msc: ['15A15', '81P15'],
  title: 'A matrix bound',
  lean: { url: library.nodes[0]!.source, revision: commit },
  pdf: { url: 'https://example.org/paper.pdf', bytes: 1, sha256: 'a'.repeat(64) },
} as CollectedPaper;

describe('Mathesis formalization presentation', () => {
  it('selects the public paper source and its real prerequisites without importing unrelated projects', () => {
    const unrelated = { ...library.nodes[0]!, id: 'Quantum/Unrelated' };
    const graph = formalizationLibrary({ ...library, nodes: [...library.nodes, unrelated] }, [
      paper,
    ]);
    expect(graph.nodes.map((node) => node.id)).toEqual(['Quantum/Base', 'Arith/Base']);
    expect(formalizationLibrary(library, []).nodes).toEqual([]);
  });
  it('uses paper MSC classifications and relates only exact shared source modules', () => {
    const graph = formalizationLibrary(library, [paper]);
    expect(graph.nodes[0]!.subjects).toEqual(['15', '81']);
    expect(graph.nodes[0]!.papers).toEqual([paper]);
    expect(graph.nodes[1]!.subjects).toEqual(['11']);
    expect(graph.nodes[1]!.papers).toEqual([]);
  });
  it('uses its own subject layout rather than upstream graph coordinates', () => {
    const graph = formalizationLibrary(library, [paper]);
    const before = layoutFormalizationLibrary(graph, '81');
    const after = layoutFormalizationLibrary(
      { ...graph, nodes: graph.nodes.map((node) => ({ ...node, x: -99, y: 12345 })) },
      '81',
    );
    expect(after).toEqual(before);
    expect(before.nodes.find((node) => node.id === 'Quantum/Base')!.x).toBeGreaterThan(
      before.nodes.find((node) => node.id === 'Arith/Base')!.x,
    );
    expect(layoutFormalizationLibrary(graph, '').nodes).toEqual([]);
  });
  it('retains required foundations across subjects without pulling in their unrelated results or later uses', () => {
    const arithmeticPaper = {
      ...paper,
      id: 'arithmetic-paper',
      msc: ['11A41'],
      lean: {
        ...paper.lean,
        url: `https://github.com/the-omega-institute/trureturing/blob/${commit}/Arith/Other.lean`,
      },
    } as CollectedPaper;
    const unrelated = {
      ...library.nodes[1]!,
      id: 'Arith/Other',
      source: arithmeticPaper.lean!.url,
    };
    const graph = formalizationLibrary(
      {
        ...library,
        nodes: [...library.nodes, unrelated],
        edges: [...library.edges, { from: 'Arith/Base', to: 'Arith/Other' }],
      },
      [paper, arithmeticPaper],
    );
    const quantum = subjectFormalizationLibrary(graph, '81');
    expect(quantum.nodes.map((node) => node.id)).toEqual(['Quantum/Base', 'Arith/Base']);
    expect(quantum.edges).toEqual([{ from: 'Arith/Base', to: 'Quantum/Base' }]);
    expect(subjectFormalizationLibrary(graph, '11').nodes.map((node) => node.id)).toEqual([
      'Arith/Base',
      'Arith/Other',
    ]);
    expect(subjectFormalizationLibrary(graph, '99').nodes).toEqual([]);
  });
  it('shows current version proof targets without turning source associations into proof dependencies', () => {
    const knowledge = {
      work: paper.id,
      version: 2,
      statements: [{ id: 'C1', name: 'Theorem 1', kind: 'theorem', statement: 'A sharper bound.' }],
      formalizations: [
        {
          claim: 'C1',
          version: 2,
          state: 'open',
          target_digest: 'sha256:' + 'c'.repeat(64),
          proofs: [],
        },
      ],
    } as unknown as CollectedKnowledge;
    const graph = formalizationLibrary(library, [paper], [knowledge]);
    const result = graph.nodes.find((node) => node.claim === 'C1')!;
    expect(result.formalized).toBe(false);
    expect(result.subjects).toEqual(['15', '81']);
    expect(graph.edges.at(-1)).toEqual({ from: 'Quantum/Base', to: result.id, context: true });
    expect(
      formalizationLibrary(library, [{ ...paper, revision: '3' }], [knowledge]).nodes,
    ).toHaveLength(2);
  });
  it('does not classify a list of linked GitHub requests as verified or currently open results', () => {
    const withLinks = {
      ...paper,
      materials: {
        formalization_requests: [
          {
            label: 'Theorem 2',
            url: 'https://github.com/the-omega-institute/trureturing-requests/issues/274',
          },
        ],
      },
    } as CollectedPaper;
    expect(formalizationLibrary(library, [withLinks]).nodes).toHaveLength(2);
  });
});
