import type { CollectedPaper, CollectedKnowledge } from '../api/types';
import { findSubject } from './subjects';
import { publicationSubjects } from './projectNoteSubjects';
import { sharedLibraryFocus, type SharedLibrary, type LibraryNode } from './sharedLibrary';

// Editorial browsing only. Paper-supplied MSC codes take precedence over these
// broad topic translations; none of them asserts statement-level proof coverage.
const topicSubjects: Record<string, string> = {
  Combinatorics: '05',
  Words: '05',
  Automata: '68',
  Rewriting: '68',
  Arith: '11',
  Digit: '11',
  Factorization: '11',
  PrimeForms: '11',
  PrimeGaps: '11',
  ArithSums: '11',
  ArithUnits: '11',
  CharacterSums: '11',
  Recurrence: '39',
  Analytic: '26',
  AnalyticClosure: '26',
  Zeros: '30',
  Asymptotics: '41',
  Constants: '33',
  Fourier: '42',
  FiniteGroups: '20',
  CayleyGrowth: '20',
  HomologicalAlgebra: '18',
  VertexAlgebra: '17',
  Weil: '14',
  Geometry: '51',
  FiniteGeometry: '51',
  Topology: '54',
  SpectralTopology: '54',
  Solenoid: '54',
  Dynamics: '37',
  FixedPoints: '47',
  Quantum: '81',
  QuantumBounds: '81',
  QuantumChannels: '81',
  QuantumStates: '81',
  QuantumContext: '81',
  StatisticalMechanics: '82',
  FluidDynamics: '76',
  Estimation: '62',
  TotalVariation: '60',
  Entropy: '94',
  RenyiDivergence: '94',
  QuadraticForms: '15',
  Eigenstructure: '15',
  Diagonal: '15',
  HilbertOperators: '47',
  Computability: '03',
};
export type FormalizationNode = LibraryNode & {
  subjects: string[];
  papers: CollectedPaper[];
  claim?: string;
};
export type FormalizationLibrary = Omit<SharedLibrary, 'nodes' | 'edges'> & {
  nodes: FormalizationNode[];
  edges: { from: string; to: string; context?: boolean }[];
};
const codes = (paper: CollectedPaper) => [
  ...new Set(
    publicationSubjects(paper.id, paper.msc)
      .map((code) => code.slice(0, 2))
      .filter((code) => findSubject(code)),
  ),
];

/** Adapt published evidence into Mathesis's paper subjects and contribution work.
 * Source associations stay distinct from Lean dependencies and carry no points.
 */
export function formalizationLibrary(
  library: SharedLibrary,
  papers: CollectedPaper[],
  knowledge: CollectedKnowledge[] = [],
  focus?: string,
): FormalizationLibrary {
  const byModule = new Map<string, CollectedPaper[]>();
  for (const paper of papers) {
    const module = sharedLibraryFocus(paper.lean?.url)?.slice(7);
    if (module) byModule.set(module, [...(byModule.get(module) ?? []), paper]);
  }
  const included = new Set([...byModule.keys(), ...(focus ? [focus] : [])]);
  const prerequisites = new Map<string, string[]>();
  for (const edge of library.edges) {
    const list = prerequisites.get(edge.to) ?? [];
    list.push(edge.from);
    prerequisites.set(edge.to, list);
  }
  const queue = [...included];
  for (let i = 0; i < queue.length; i++)
    for (const id of prerequisites.get(queue[i]!) ?? []) {
      if (!included.has(id)) {
        included.add(id);
        queue.push(id);
      }
    }
  const nodes: FormalizationNode[] = library.nodes
    .filter((node) => included.has(node.id))
    .map((node) => {
      const associated = byModule.get(node.id) ?? [];
      const supplied = [...new Set(associated.flatMap(codes))];
      return {
        ...node,
        subjects: supplied.length ? supplied : [topicSubjects[node.domain] ?? '00'],
        papers: associated,
      };
    });
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const edges: FormalizationLibrary['edges'] = library.edges.filter(
    (edge) => byId.has(edge.from) && byId.has(edge.to),
  );
  for (const item of knowledge) {
    const paper = papers.find(
      (paper) => paper.id === item.work && paper.revision === String(item.version),
    );
    if (!paper) continue;
    const source = sharedLibraryFocus(paper.lean?.url)?.slice(7);
    for (const target of item.formalizations ?? []) {
      if (!target.target_digest || target.version !== item.version || target.state === 'disproved')
        continue;
      const claim = item.statements.find((claim) => claim.id === target.claim);
      if (!claim) continue;
      const formalized =
        target.state === 'verified' &&
        target.proofs.some(
          (proof) =>
            proof.receipt.target_digest === target.target_digest &&
            proof.receipt.verdict === 'proved',
        );
      const id = `statement:${paper.id}:${paper.revision}:${claim.id}`;
      nodes.push({
        id,
        title: claim.title || claim.name,
        abstract: claim.statement,
        domain: '',
        source: paper.pdf.url,
        formalized,
        x: 0,
        y: 0,
        subjects: codes(paper).length ? codes(paper) : ['00'],
        papers: [paper],
        claim: claim.id,
      });
      // Context locates a target within its paper; it is not a proof dependency.
      if (source && byId.has(source)) edges.push({ from: source, to: id, context: true });
    }
  }
  return { ...library, nodes, edges };
}

/** Mathesis lays out each subject independently, without copying upstream coordinates. */
export function layoutFormalizationLibrary(
  library: FormalizationLibrary,
  subject: string,
): { nodes: FormalizationNode[]; labels: { code: string; x: number; y: number }[] } {
  const groups = new Map<string, FormalizationNode[]>();
  for (const node of library.nodes) {
    if (subject && !node.subjects.includes(subject)) continue;
    const code = subject || node.subjects[0]!;
    groups.set(code, [...(groups.get(code) ?? []), node]);
  }
  const labels: { code: string; x: number; y: number }[] = [];
  const nodes: FormalizationNode[] = [];
  const columns = subject ? 1 : Math.min(6, Math.ceil(Math.sqrt(groups.size * 1.6)));
  [...groups]
    .sort(([a], [b]) => a.localeCompare(b))
    .forEach(([code, items], index) => {
      const originX = (index % columns) * 300;
      const originY = Math.floor(index / columns) * 230;
      labels.push({ code, x: originX, y: originY });
      const ids = new Set(items.map((node) => node.id));
      const links = library.edges.filter((edge) => ids.has(edge.from) && ids.has(edge.to));
      const indegree = new Map(items.map((node) => [node.id, 0]));
      const depth = new Map(items.map((node) => [node.id, 0]));
      for (const edge of links) indegree.set(edge.to, indegree.get(edge.to)! + 1);
      const ready = items
        .filter((node) => !indegree.get(node.id))
        .map((node) => node.id)
        .sort();
      for (let i = 0; i < ready.length; i++)
        for (const edge of links.filter((edge) => edge.from === ready[i])) {
          depth.set(edge.to, Math.max(depth.get(edge.to)!, depth.get(edge.from)! + 1));
          indegree.set(edge.to, indegree.get(edge.to)! - 1);
          if (!indegree.get(edge.to)) ready.push(edge.to);
        }
      const layers = new Map<number, FormalizationNode[]>();
      for (const node of items) {
        const rank = depth.get(node.id)!;
        const list = layers.get(rank) ?? [];
        list.push(node);
        layers.set(rank, list);
      }
      const maxDepth = Math.max(...layers.keys(), 1);
      for (const [rank, layer] of layers)
        layer
          .sort((a, b) => a.id.localeCompare(b.id))
          .forEach((node, i) => {
            nodes.push({
              ...node,
              x: originX + 20 + ((i + 0.5) * 240) / layer.length,
              y: originY + 85 + (rank * 120) / maxDepth,
            });
          });
    });
  return { nodes, labels };
}
