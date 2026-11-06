import type { ApiClient } from '../api/client';
import type { Entrant, ResultUse } from '../api/types';
import { researchAreas, theoryAreas } from './theoryLibrary';

export type DependencyNode = {
  id: string;
  kind: 'result' | 'paper' | 'literature' | 'contribution';
  title: string;
  label?: string;
  statement?: string;
  authors: string[];
  work?: string;
  version?: string;
  claim?: string;
  collected?: boolean;
  paper?: string;
  section?: string;
  areas: readonly string[];
  macros?: Record<string, string>;
  verified?: boolean;
  target?: boolean;
  url?: string;
  leanSource?: string;
  contributor?: Entrant;
};
export type DependencyRelation =
  'manuscript_reference' | 'declared_dependency' | 'checked_use' | 'cites' | 'contains';
export type DependencyEdge = { from: string; to: string; relation: DependencyRelation };
export type DependencySnapshot = {
  nodes: DependencyNode[];
  edges: DependencyEdge[];
  next: string | null;
};
const statementId = (work: string, version: string, claim: string) =>
  `statement:${work}:${version}:${claim}`;

/** Read existing Rust-owned public projections. No review, indexing or proof jobs. */
export async function loadDependencyGraph(
  api: ApiClient,
  before: string | undefined,
  signal: AbortSignal,
): Promise<DependencySnapshot> {
  const source = await api.researchGraph({ limit: 20, before }, { signal });
  const identities = new Map(
    source.nodes.map((node) => [
      node.id,
      node.kind === 'manuscript' && node.version ? `${node.id}:${node.version}` : node.id,
    ]),
  );
  const nodes: DependencyNode[] = source.nodes.map((node) => ({
    id: identities.get(node.id)!,
    kind: node.kind === 'literature' ? 'literature' : 'paper',
    title: node.title,
    authors: node.authors,
    work: node.record ?? undefined,
    version: node.version ? String(node.version) : undefined,
    areas: [],
    url: node.url ?? undefined,
  }));
  // Citation direction is reversed only for reading: earlier source -> citing work.
  const edges: DependencyEdge[] = source.edges.map((edge) => ({
    from: identities.get(edge.to)!,
    to: identities.get(edge.from)!,
    relation: 'cites',
  }));
  const papers = nodes.filter((node) => node.kind === 'paper' && node.work && node.version);
  await Promise.all(
    papers.map(async (node) => {
      const work = node.work!;
      const version = node.version!;
      const collected = !work.startsWith('WP-');
      node.collected = collected;
      if (collected) {
        const [paper, knowledge] = await Promise.all([
          api.getCollectedPaper(work, version, { signal }),
          api.collectedKnowledge(work, version, { signal }),
        ]);
        node.areas = researchAreas(work, paper.msc);
        node.leanSource = paper.lean?.url;
        for (const claim of knowledge.statements) {
          const id = statementId(work, version, claim.id);
          nodes.push({
            id,
            kind: 'result',
            title: claim.title ?? claim.section ?? claim.name,
            label: claim.name,
            statement: claim.statement,
            authors: node.authors,
            work,
            version,
            claim: claim.id,
            collected,
            paper: node.id,
            section: claim.section ?? undefined,
            areas: node.areas,
            macros: knowledge.macros,
          });
          edges.push({ from: node.id, to: id, relation: 'contains' });
        }
        for (const edge of knowledge.connections)
          edges.push({
            from: statementId(work, version, edge.to),
            to: statementId(work, version, edge.from),
            relation: 'manuscript_reference',
          });
      } else {
        const paper = await api.getPaper(work, { signal });
        // A revoked/legacy publication may return only bibliographic metadata.
        // Its statements and dependencies must never be reconstructed from a review.
        if (!paper.publication || String(paper.publication.version) !== version) return;
        const acceptedVersion = String(paper.publication.version);
        node.version = acceptedVersion;
        node.areas = researchAreas(work, paper.summary.msc);
        for (const claim of paper.claims ?? []) {
          const id = statementId(work, acceptedVersion, claim.id);
          nodes.push({
            id,
            kind: 'result',
            title: claim.section ?? claim.label.replace(/\s*\[[^\]]*\]\s*$/, ''),
            label: claim.label.replace(/\s*\[[^\]]*\]\s*$/, ''),
            statement: claim.statement,
            authors: node.authors,
            work,
            version: acceptedVersion,
            claim: claim.id,
            collected,
            paper: node.id,
            section: claim.section,
            areas: node.areas,
            macros: paper.macros,
            verified: Boolean(claim.verified),
            target: Boolean(paper.lean_statements?.some((target) => target.claim === claim.id)),
          });
          edges.push({ from: node.id, to: id, relation: 'contains' });
          for (const dependency of claim.depends_on)
            edges.push({
              from: statementId(work, acceptedVersion, dependency),
              to: id,
              relation: 'declared_dependency',
            });
        }
      }
    }),
  );
  return cleanSnapshot({ nodes, edges, next: source.next_before });
}

export function cleanSnapshot(graph: DependencySnapshot): DependencySnapshot {
  const nodes = [...new Map(graph.nodes.map((node) => [node.id, node])).values()];
  const ids = new Set(nodes.map((node) => node.id));
  const edges = graph.edges.filter((edge) => ids.has(edge.from) && ids.has(edge.to));
  return {
    ...graph,
    nodes: nodes.sort((a, b) => a.id.localeCompare(b.id)),
    edges: [...new Map(edges.map((e) => [JSON.stringify(e), e])).values()],
  };
}

/** Only the existing proof-term inspection API may add checked-use arrows. */
export function addCheckedUses(graph: DependencySnapshot, uses: ResultUse[]): DependencySnapshot {
  const nodes = [...graph.nodes];
  const edges = [...graph.edges];
  for (const use of uses) {
    if (use.basis !== 'checked_proof_term') continue;
    const target = statementId(use.result.record, String(use.result.version), use.result.claim);
    if (!nodes.some((node) => node.id === target)) continue;
    const id = `contribution:${use.source_attempt}:${use.contribution.name}`;
    nodes.push({
      id,
      kind: 'contribution',
      title: use.contribution.title,
      statement: use.contribution.proposition,
      authors: use.used_result.authors.map((a) => a.name),
      work: use.used_result.record,
      version: String(use.used_result.version),
      claim: use.used_result.claim,
      areas: researchAreas(use.used_result.record),
      verified: true,
      contributor: use.contributor,
    });
    edges.push({ from: id, to: target, relation: 'checked_use' });
  }
  return cleanSnapshot({ ...graph, nodes, edges });
}

export function resultDegree(graph: DependencySnapshot, id: string) {
  return graph.edges.filter(
    (edge) =>
      edge.relation !== 'contains' &&
      edge.relation !== 'cites' &&
      (edge.from === id || edge.to === id),
  ).length;
}

/** A deterministic layered layout, condensing cycles rather than pretending all source references form a DAG. */
export function dependencyLayout(graph: DependencySnapshot) {
  const successors = new Map(graph.nodes.map((node) => [node.id, [] as string[]]));
  for (const edge of graph.edges) successors.get(edge.from)?.push(edge.to);
  const index = new Map<string, number>();
  const low = new Map<string, number>();
  const stack: string[] = [];
  const active = new Set<string>();
  const component = new Map<string, number>();
  let serial = 0;
  let components = 0;
  function visit(id: string) {
    index.set(id, serial);
    low.set(id, serial++);
    stack.push(id);
    active.add(id);
    for (const child of successors.get(id) ?? []) {
      if (!index.has(child)) {
        visit(child);
        low.set(id, Math.min(low.get(id)!, low.get(child)!));
      } else if (active.has(child)) low.set(id, Math.min(low.get(id)!, index.get(child)!));
    }
    if (low.get(id) === index.get(id)) {
      let child: string;
      do {
        child = stack.pop()!;
        active.delete(child);
        component.set(child, components);
      } while (child !== id);
      components++;
    }
  }
  for (const node of graph.nodes) if (!index.has(node.id)) visit(node.id);
  const incoming = Array.from({ length: components }, () => new Set<number>());
  for (const edge of graph.edges) {
    const from = component.get(edge.from)!;
    const to = component.get(edge.to)!;
    if (from !== to) incoming[to]!.add(from);
  }
  const ranks = new Map<number, number>();
  function rank(id: number): number {
    const saved = ranks.get(id);
    if (saved !== undefined) return saved;
    const value = Math.max(0, ...[...incoming[id]!].map((parent) => rank(parent) + 1));
    ranks.set(id, value);
    return value;
  }
  const rows = new Map<number, DependencyNode[]>();
  for (const node of graph.nodes) {
    const column = rank(component.get(node.id)!);
    rows.set(column, [...(rows.get(column) ?? []), node]);
  }
  const points = new Map<string, { x: number; y: number }>();
  let columnOffset = 0;
  const rowLimit = 12;
  for (const [, list] of [...rows.entries()].sort(([a], [b]) => a - b)) {
    list.sort(
      (a, b) => (a.paper ?? a.id).localeCompare(b.paper ?? b.id) || a.id.localeCompare(b.id),
    );
    list.forEach((node, row) =>
      points.set(node.id, {
        x: 40 + (columnOffset + Math.floor(row / rowLimit)) * 320,
        y: 40 + (row % rowLimit) * 110,
      }),
    );
    columnOffset += Math.max(1, Math.ceil(list.length / rowLimit));
  }
  return {
    points,
    width: 80 + columnOffset * 320,
    height:
      140 + Math.min(rowLimit, Math.max(1, ...[...rows.values()].map((list) => list.length))) * 110,
  };
}

/** A field overview positions the same exact results; grouping creates no dependency edges. */
export function dependencyAtlasLayout(graph: DependencySnapshot, availableWidth: number) {
  const columns = availableWidth >= 1160 ? 3 : availableWidth >= 720 ? 2 : 1;
  const tileWidth = availableWidth / columns;
  const tileHeight = 320;
  const groups = new Map<string, DependencyNode[]>();
  for (const node of graph.nodes) {
    if (node.kind !== 'result' && node.kind !== 'contribution') continue;
    const area = node.areas[0] ?? 'unclassified';
    groups.set(area, [...(groups.get(area) ?? []), node]);
  }
  const points = new Map<string, { x: number; y: number }>();
  const fieldOrder = (id: string) => {
    const index = theoryAreas.findIndex((area) => area.id === id);
    return index < 0 ? theoryAreas.length : index;
  };
  const fields = [...groups]
    .sort(([a], [b]) => fieldOrder(a) - fieldOrder(b))
    .map(([id, nodes], index) => {
      const x = (index % columns) * tileWidth;
      const y = Math.floor(index / columns) * tileHeight;
      const ids = new Set(nodes.map((node) => node.id));
      const local = dependencyLayout({
        ...graph,
        nodes,
        edges: graph.edges.filter((edge) => ids.has(edge.from) && ids.has(edge.to)),
      });
      const scale = Math.min((tileWidth - 64) / local.width, (tileHeight - 96) / local.height);
      for (const node of nodes) {
        const p = local.points.get(node.id)!;
        points.set(node.id, { x: x + 24 + (p.x + 120) * scale, y: y + 72 + p.y * scale });
      }
      return {
        id,
        x,
        y,
        width: tileWidth - 40,
        count: nodes.filter((node) => node.kind === 'result').length,
      };
    });
  return {
    points,
    fields,
    width: availableWidth,
    height: Math.max(tileHeight, Math.ceil(fields.length / columns) * tileHeight),
  };
}
