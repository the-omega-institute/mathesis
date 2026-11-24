import type { CollectedPaper, CollectedKnowledge, SharedFormalizationRequest } from '../api/types';
import { findSubject } from './subjects';
import { publicationSubjects } from './projectNoteSubjects';
import { sharedLibraryFocus, type SharedLibrary, type LibraryNode } from './sharedLibrary';

// Knowledge follows its mathematical topic. Paper metadata is only a fallback
// for an unclassified source; these browsing labels assert no proof coverage.
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
  request?: SharedFormalizationRequest;
};
export type FormalizationLibrary = Omit<SharedLibrary, 'nodes' | 'edges'> & {
  nodes: FormalizationNode[];
  edges: { from: string; to: string; context?: boolean }[];
};
export const formalizationPaperSubjects = (paper: { id: string; msc?: string[] }) => [
  ...new Set(
    publicationSubjects(paper.id, paper.msc)
      .map((code) => code.slice(0, 2))
      .filter((code) => findSubject(code)),
  ),
];

/** Present the whole published knowledge base, independently of the paper archive.
 * Paper links enrich a node; they never determine which mathematics exists.
 */
export function formalizationLibrary(
  library: SharedLibrary,
  papers: CollectedPaper[],
  knowledge: CollectedKnowledge[] = [],
  relatedPapers: CollectedPaper[] = papers,
  requests: SharedFormalizationRequest[] = [],
): FormalizationLibrary {
  const byModule = new Map<string, CollectedPaper[]>();
  for (const paper of relatedPapers) {
    const module = sharedLibraryFocus(paper.lean?.url)?.slice(7);
    if (module) byModule.set(module, [...(byModule.get(module) ?? []), paper]);
  }
  const nodes: FormalizationNode[] = library.nodes.map((node) => {
    const associated = byModule.get(node.id) ?? [];
    const supplied = [...new Set(associated.flatMap(formalizationPaperSubjects))];
    const topic = topicSubjects[node.domain];
    return {
      ...node,
      subjects: topic ? [topic] : supplied.length ? supplied : ['00'],
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
        subjects: formalizationPaperSubjects(paper).length
          ? formalizationPaperSubjects(paper)
          : ['00'],
        papers: [paper],
        claim: claim.id,
      });
      // Context locates a target within its paper; it is not a proof dependency.
      if (source && byId.has(source)) edges.push({ from: source, to: id, context: true });
    }
  }
  for (const request of requests) {
    if (!['open', 'claimed'].includes(request.state)) continue;
    const source = sharedLibraryFocus(request.lean_source ?? undefined)?.slice(7);
    const context = source ? byId.get(source) : undefined;
    const id = `request:${request.number}`;
    nodes.push({
      id,
      title: request.title,
      abstract: '',
      domain: context?.domain ?? '',
      source: request.url,
      formalized: false,
      x: 0,
      y: 0,
      subjects: context?.subjects ?? ['00'],
      papers: [],
      request,
    });
    if (source && context) edges.push({ from: source, to: id, context: true });
  }
  return { ...library, nodes, edges };
}

/** One subject's mathematics and its actual prerequisites, never the union of fields. */
export function subjectFormalizationLibrary(
  library: FormalizationLibrary,
  subject: string,
): FormalizationLibrary {
  if (!findSubject(subject)) return { ...library, nodes: [], edges: [] };
  const included = new Set(
    library.nodes.filter((node) => node.subjects.includes(subject)).map((node) => node.id),
  );
  const prerequisites = new Map<string, string[]>();
  for (const edge of library.edges) {
    if (edge.context) continue;
    prerequisites.set(edge.to, [...(prerequisites.get(edge.to) ?? []), edge.from]);
  }
  const queue = [...included];
  for (let i = 0; i < queue.length; i++)
    for (const id of prerequisites.get(queue[i]!) ?? []) {
      if (included.has(id)) continue;
      included.add(id);
      queue.push(id);
    }
  return {
    ...library,
    nodes: library.nodes.filter((node) => included.has(node.id)),
    edges: library.edges.filter((edge) => included.has(edge.from) && included.has(edge.to)),
  };
}

/** Prefer the result's mathematical topic when it is also a recorded paper subject. */
export function primaryFormalizationSubject(node: FormalizationNode): string | undefined {
  const topic = topicSubjects[node.domain];
  return topic && node.subjects.includes(topic) ? topic : node.subjects[0];
}

/** Lay out one subject's DAG from its own edges; source coordinates are not used. */
export function layoutFormalizationLibrary(
  library: FormalizationLibrary,
  subject: string,
): { nodes: FormalizationNode[] } {
  const graph = subjectFormalizationLibrary(library, subject);
  const outgoing = new Map<string, string[]>();
  const indegree = new Map(graph.nodes.map((node) => [node.id, 0]));
  const depth = new Map(graph.nodes.map((node) => [node.id, 0]));
  for (const edge of graph.edges) {
    if (edge.context) continue;
    outgoing.set(edge.from, [...(outgoing.get(edge.from) ?? []), edge.to]);
    indegree.set(edge.to, indegree.get(edge.to)! + 1);
  }
  const ready = graph.nodes
    .filter((node) => !indegree.get(node.id))
    .map((node) => node.id)
    .sort();
  for (let i = 0; i < ready.length; i++)
    for (const to of outgoing.get(ready[i]!) ?? []) {
      depth.set(to, Math.max(depth.get(to)!, depth.get(ready[i]!)! + 1));
      indegree.set(to, indegree.get(to)! - 1);
      if (!indegree.get(to)) ready.push(to);
    }
  // A paper-context link places its exact targets after its source module,
  // without treating that association as a mathematical proof dependency.
  for (const edge of graph.edges)
    if (edge.context) depth.set(edge.to, Math.max(depth.get(edge.to)!, depth.get(edge.from)! + 1));
  const layers = new Map<number, FormalizationNode[]>();
  for (const node of graph.nodes) {
    const rank = depth.get(node.id)!;
    layers.set(rank, [...(layers.get(rank) ?? []), node]);
  }
  const nodes: FormalizationNode[] = [];
  for (const [rank, layer] of layers)
    layer
      .sort((a, b) => a.id.localeCompare(b.id))
      .forEach((node, index) => {
        nodes.push({ ...node, x: rank * 200, y: (index - (layer.length - 1) / 2) * 36 });
      });
  return { nodes };
}
