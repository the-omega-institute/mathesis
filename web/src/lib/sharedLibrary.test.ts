import { webcrypto } from 'node:crypto';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { parseSharedLibrary, sha256, sharedLibraryFocus } from './sharedLibrary';
const hash = 'sha256:' + 'a'.repeat(64);
const commit = 'b'.repeat(40);
beforeEach(() => vi.stubGlobal('crypto', webcrypto));
afterEach(() => vi.unstubAllGlobals());
async function fixture() {
  const row = (id: string, status: string, lean: unknown, kind = 'truth') => [
    id,
    kind,
    0,
    id + ' title',
    'A precise mathematical result.',
    'Algebra',
    status,
    0,
    0,
    null,
    null,
    0,
    0,
    0,
    0,
    0,
    lean,
    131071,
  ];
  const nodes = [
    row('D5/Proved', 'Closed', {
      eligible: true,
      freeze_status: 'frozen',
      frozen_node_id: 0,
      source_sha256: 0,
      node_axiom_closure: ['propext'],
      declaration_count: 2,
      theorem_count: 1,
    }),
    row('D5/Open', 'Open', { eligible: false }),
    row('D5/Uninspected', 'Closed', { eligible: false }),
    row('Blueprint/Story', 'Open', null, 'blueprint'),
  ];
  const snapshot = {
    source_commit: commit,
    source_tree: commit,
    truth_release_digest: hash,
    truth_graph_sha256: hash,
  };
  const admission = {
    schema_version: 'pages-atlas-admission.v1',
    ...snapshot,
    atlas_graph_digest: hash,
    coverage: 'available',
    evidence: { truth_graph_sha256: hash, truth_export_sha256: hash, raw_lean_report_sha256: hash },
    source_truth_nodes: 3,
    admitted: 1,
    excluded: 2,
  };
  const manifest = {
    schema_version: 'pages-atlas-manifest.v1',
    ...snapshot,
    atlas_graph_digest: hash,
    counts: { nodes: 4, truth_nodes: 3, edges: 4 },
  };
  const view = {
    schema_version: 'pages-atlas-startup-view.v3',
    atlas_graph_digest: hash,
    truth_release_digest: hash,
    source_snapshot: snapshot,
    math_admission: admission,
    nodes,
    lean_hashes: btoa('\0'.repeat(64)),
    edges: [
      [0, 1, 'truth-dependency', null],
      [0, 1, 'structural-affinity', null],
      [2, 0, 'truth-dependency', null],
      [0, 1, 'truth-dependency', 'proposed'],
    ],
  };
  const layout = {
    schema_version: 'pages-public-layout-indexed.v1',
    atlas_graph_digest: hash,
    truth_release_digest: hash,
    node_order_digest: await sha256(
      new TextEncoder().encode(JSON.stringify(nodes.map((row) => row[0]))),
    ),
    dependency_positions: [
      [0, 1000, 0],
      [1000, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
    ],
  };
  return { manifest, view, layout };
}
describe('shared published Lean graph', () => {
  it('links only pinned public library source paths to module identities', () => {
    expect(
      sharedLibraryFocus(
        `https://github.com/the-omega-institute/trureturing/blob/${commit}/D5/Proved.lean#L10`,
      ),
    ).toBe('module:D5/Proved');
    expect(
      sharedLibraryFocus(
        'https://github.com/the-omega-institute/trureturing/blob/dev/D5/Proved.lean',
      ),
    ).toBeUndefined();
    expect(
      sharedLibraryFocus(`https://github.com/other/repository/blob/${commit}/D5/Proved.lean`),
    ).toBeUndefined();
  });
  it('retains real formalizations and explicit open nodes while excluding affinities and uninspected sources', async () => {
    const f = await fixture();
    const graph = await parseSharedLibrary(f.manifest, f.view, f.layout);
    expect(graph.nodes.map((n) => [n.id, n.formalized])).toEqual([
      ['D5/Proved', true],
      ['D5/Open', false],
    ]);
    expect(graph.edges).toEqual([{ from: 'D5/Proved', to: 'D5/Open' }]);
    expect(graph.nodes[0]!.source).toBe(
      `https://github.com/the-omega-institute/trureturing/blob/${commit}/D5/Proved.lean`,
    );
  });
  it('refuses mixed releases, changed source identities, incomplete layouts and inconsistent admission counts', async () => {
    const f = await fixture();
    await expect(
      parseSharedLibrary(
        f.manifest,
        { ...f.view, truth_release_digest: 'sha256:' + 'c'.repeat(64) },
        f.layout,
      ),
    ).rejects.toThrow();
    await expect(
      parseSharedLibrary(
        f.manifest,
        {
          ...f.view,
          source_snapshot: { ...f.view.source_snapshot, source_commit: 'c'.repeat(40) },
        },
        f.layout,
      ),
    ).rejects.toThrow();
    await expect(
      parseSharedLibrary(f.manifest, f.view, {
        ...f.layout,
        node_order_digest: 'sha256:' + 'c'.repeat(64),
      }),
    ).rejects.toThrow();
    await expect(
      parseSharedLibrary(
        f.manifest,
        { ...f.view, math_admission: { ...f.view.math_admission, admitted: 2 } },
        f.layout,
      ),
    ).rejects.toThrow();
  });
  it('rejects unsafe proof evidence and missing dependency endpoints', async () => {
    const f = await fixture();
    const changed = structuredClone(f.view);
    (changed.nodes[0]![16] as Record<string, unknown>).node_axiom_closure = ['sorryAx'];
    await expect(parseSharedLibrary(f.manifest, changed, f.layout)).rejects.toThrow();
    await expect(
      parseSharedLibrary(
        f.manifest,
        { ...f.view, edges: [[0, 99, 'truth-dependency', null], ...f.view.edges.slice(1)] },
        f.layout,
      ),
    ).rejects.toThrow();
  });
});
