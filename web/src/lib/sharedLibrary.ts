/** Read the existing public Atlas contract. No Lean build or proof job is started. */
const BASE = 'https://www.trureturing.org/';
const HASH = /^sha256:[a-f0-9]{64}$/;
const COMMIT = /^[a-f0-9]{40}$/;
const AXIOMS = new Set(['propext', 'Classical.choice', 'Quot.sound']);
const FIELDS = [
  'id',
  'kind',
  'title',
  'human_title',
  'human_abstract',
  'domain',
  'status',
  'state',
  'repo_path',
  'structure_source',
  'structural_role',
  'out_degree',
  'true_depth',
  'descendant_count',
  'descendant_cost',
  'dependency_betweenness',
  'lean',
];
const DEPENDENCIES = new Set([
  'truth-dependency',
  'module-import',
  'frozen-prerequisite',
  'dependency',
]);

type Json = Record<string, unknown>;
type Artifact = { path: string; digest: string; bytes: number };
export type LibraryNode = {
  id: string;
  title: string;
  abstract: string;
  domain: string;
  source: string;
  formalized: boolean;
  x: number;
  y: number;
};
export type SharedLibrary = {
  nodes: LibraryNode[];
  edges: { from: string; to: string }[];
  sourceCommit: string;
  digest: string;
};
function requireValue(condition: unknown): asserts condition {
  if (!condition) throw new Error('The published mathematical library could not be verified.');
}
function object(value: unknown): Json {
  requireValue(value && typeof value === 'object' && !Array.isArray(value));
  return value as Json;
}
function text(value: unknown): string {
  requireValue(typeof value === 'string');
  return value;
}
export async function sha256(bytes: BufferSource): Promise<string> {
  return (
    'sha256:' +
    Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), (b) =>
      b.toString(16).padStart(2, '0'),
    ).join('')
  );
}
async function jsonResponse(response: Response): Promise<unknown> {
  if (!response.ok)
    throw new Error(`The shared library is temporarily unavailable (HTTP ${response.status}).`);
  requireValue(Number(response.headers.get('content-length') ?? 0) <= 32 * 1024 * 1024);
  const body = await response.text();
  requireValue(body.length <= 32 * 1024 * 1024);
  return JSON.parse(body) as unknown;
}
async function artifact(value: unknown, signal: AbortSignal): Promise<unknown> {
  const entry = object(value) as unknown as Artifact;
  requireValue(
    HASH.test(entry.digest) &&
      entry.path === `data/atlas-startup/${entry.digest.slice(7)}.json.gz` &&
      Number.isInteger(entry.bytes) &&
      entry.bytes > 0 &&
      entry.bytes <= 32 * 1024 * 1024,
  );
  const response = await fetch(new URL(entry.path, BASE), { signal, cache: 'force-cache' });
  requireValue(response.ok);
  const bytes = await response.arrayBuffer();
  requireValue(bytes.byteLength <= 16 * 1024 * 1024 && (await sha256(bytes)) === entry.digest);
  const decompressed = new Response(
    new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip')),
  );
  return jsonResponse(decompressed);
}

/** The source graph distinguishes actual dependencies from editorial affinities. */
export async function parseSharedLibrary(
  manifestValue: unknown,
  viewValue: unknown,
  layoutValue: unknown,
): Promise<SharedLibrary> {
  const manifest = object(manifestValue),
    view = object(viewValue),
    layout = object(layoutValue);
  const snapshot = object(view.source_snapshot),
    admission = object(view.math_admission);
  requireValue(
    manifest.schema_version === 'pages-atlas-manifest.v1' &&
      view.schema_version === 'pages-atlas-startup-view.v3' &&
      layout.schema_version === 'pages-public-layout-indexed.v1',
  );
  requireValue(
    HASH.test(text(manifest.atlas_graph_digest)) &&
      HASH.test(text(manifest.truth_release_digest)) &&
      COMMIT.test(text(manifest.source_commit)),
  );
  requireValue(
    view.atlas_graph_digest === manifest.atlas_graph_digest &&
      view.truth_release_digest === manifest.truth_release_digest &&
      snapshot.source_commit === manifest.source_commit &&
      snapshot.source_tree === manifest.source_tree &&
      snapshot.truth_release_digest === manifest.truth_release_digest,
  );
  requireValue(
    layout.atlas_graph_digest === manifest.atlas_graph_digest &&
      layout.truth_release_digest === manifest.truth_release_digest,
  );
  requireValue(
    admission.schema_version === 'pages-atlas-admission.v1' &&
      admission.coverage === 'available' &&
      admission.atlas_graph_digest === manifest.atlas_graph_digest &&
      admission.truth_release_digest === manifest.truth_release_digest &&
      admission.source_commit === manifest.source_commit &&
      admission.source_tree === manifest.source_tree,
  );
  const evidence = object(admission.evidence);
  requireValue(
    evidence.truth_graph_sha256 === snapshot.truth_graph_sha256 &&
      ['truth_graph_sha256', 'truth_export_sha256', 'raw_lean_report_sha256'].every((key) =>
        HASH.test(text(evidence[key])),
      ),
  );
  const rows = view.nodes,
    rawEdges = view.edges,
    positions = layout.dependency_positions ?? layout.positions;
  const counts = object(manifest.counts);
  requireValue(
    Array.isArray(rows) &&
      rows.length <= 100000 &&
      Array.isArray(rawEdges) &&
      rawEdges.length <= 1000000 &&
      rows.length === counts.nodes &&
      rawEdges.length === counts.edges &&
      Array.isArray(positions) &&
      positions.length === rows.length,
  );
  const encoded = text(view.lean_hashes);
  requireValue(/^[A-Za-z0-9+/]*={0,2}$/.test(encoded));
  const hashes = atob(encoded);
  requireValue(btoa(hashes) === encoded);
  let offset = 0,
    truthCount = 0,
    admitted = 0;
  const seen = new Set<string>();
  const nodes: LibraryNode[] = [];
  const order: string[] = [];
  for (const [index, value] of rows.entries()) {
    requireValue(Array.isArray(value) && value.length === FIELDS.length + 1);
    const mask = value[FIELDS.length];
    requireValue(Number.isInteger(mask) && mask >= 0 && mask < 2 ** FIELDS.length);
    const node = Object.fromEntries(
      FIELDS.flatMap((key, column) => (mask & (1 << column) ? [[key, value[column]]] : [])),
    );
    const id = text(node.id);
    requireValue(id && !seen.has(id));
    seen.add(id);
    order.push(id);
    const lean = node.lean ? { ...object(node.lean) } : undefined;
    if (lean)
      for (const key of ['frozen_node_id', 'source_sha256']) {
        if (lean[key] !== 0) continue;
        requireValue(offset + 32 <= hashes.length);
        lean[key] =
          'sha256:' +
          Array.from(hashes.slice(offset, offset + 32), (c) =>
            c.charCodeAt(0).toString(16).padStart(2, '0'),
          ).join('');
        offset += 32;
      }
    const point = positions[index];
    requireValue(
      Array.isArray(point) &&
        point.length === 3 &&
        point.every((v) => Number.isSafeInteger(v) && Math.abs(v) < 1e10),
    );
    if (node.kind !== 'truth') continue;
    truthCount++;
    const formalized = lean?.eligible === true;
    if (formalized) {
      requireValue(lean);
      requireValue(
        ['frozen', 'proven-not-yet-frozen'].includes(text(lean.freeze_status)) &&
          HASH.test(text(lean.frozen_node_id)) &&
          HASH.test(text(lean.source_sha256)) &&
          Number.isInteger(lean.declaration_count) &&
          Number(lean.declaration_count) > 0 &&
          Number.isInteger(lean.theorem_count) &&
          Number(lean.theorem_count) >= 0 &&
          Number(lean.theorem_count) <= Number(lean.declaration_count) &&
          Array.isArray(lean.node_axiom_closure) &&
          lean.node_axiom_closure.every((a) => typeof a === 'string' && AXIOMS.has(a)),
      );
      admitted++;
    }
    if (!formalized && String(node.state === 0 ? node.status : node.state).toLowerCase() !== 'open')
      continue;
    const path = node.repo_path === 0 ? id + '.lean' : text(node.repo_path);
    requireValue(
      path === id + '.lean' &&
        !path.split('/').some((part) => part === '..') &&
        !path.startsWith('/'),
    );
    nodes.push({
      id,
      title:
        typeof node.human_title === 'string' && node.human_title
          ? node.human_title
          : typeof node.title === 'string'
            ? node.title
            : id.split('/').at(-1)!,
      abstract: typeof node.human_abstract === 'string' ? node.human_abstract : '',
      domain: typeof node.domain === 'string' ? node.domain : 'Mathematics',
      formalized,
      x: (Number(point[0]) + Number(point[2]) * 0.45) / 1000,
      y: (-Number(point[1]) + Number(point[2]) * 0.18) / 1000,
      source: `https://github.com/the-omega-institute/trureturing/blob/${manifest.source_commit}/${path.split('/').map(encodeURIComponent).join('/')}`,
    });
  }
  requireValue(
    offset === hashes.length &&
      truthCount === counts.truth_nodes &&
      truthCount === admission.source_truth_nodes &&
      admitted === admission.admitted &&
      truthCount - admitted === admission.excluded,
  );
  requireValue(
    (await sha256(new TextEncoder().encode(JSON.stringify(order)))) === layout.node_order_digest,
  );
  const included = new Set(nodes.map((n) => n.id));
  const edges: SharedLibrary['edges'] = [];
  for (const value of rawEdges) {
    requireValue(
      Array.isArray(value) &&
        value.length === 4 &&
        Number.isInteger(value[0]) &&
        Number.isInteger(value[1]) &&
        order[value[0]] !== undefined &&
        order[value[1]] !== undefined &&
        typeof value[2] === 'string',
    );
    const from = order[value[0]]!,
      to = order[value[1]]!;
    if (
      included.has(from) &&
      included.has(to) &&
      DEPENDENCIES.has(value[2]) &&
      !['advisory', 'proposed'].includes(value[3])
    )
      edges.push({ from, to });
  }
  return {
    nodes,
    edges,
    sourceCommit: text(manifest.source_commit),
    digest: text(manifest.atlas_graph_digest),
  };
}
export async function loadSharedLibrary(signal: AbortSignal): Promise<SharedLibrary> {
  const [manifestValue, startupValue] = await Promise.all([
    fetch(BASE + 'data/pages-atlas-manifest.v1.json', { signal, cache: 'no-cache' }).then(
      jsonResponse,
    ),
    fetch(BASE + 'data/atlas-startup.v1.json', { signal, cache: 'no-cache' }).then(jsonResponse),
  ]);
  const manifest = object(manifestValue),
    startup = object(startupValue);
  requireValue(
    startup.schema_version === 'pages-atlas-startup.v1' &&
      startup.atlas_graph_digest === manifest.atlas_graph_digest &&
      startup.truth_release_digest === manifest.truth_release_digest,
  );
  const fast = object(startup.fast);
  const [graph, layout] = await Promise.all([
    artifact(fast.graph, signal),
    artifact(fast.layout, signal),
  ]);
  signal.throwIfAborted();
  return parseSharedLibrary(manifest, graph, layout);
}

/** A source association keeps the paper version and proof revision distinct. */
export function sharedLibraryFocus(source?: string): string | undefined {
  if (!source) return;
  try {
    const url = new URL(source);
    const match = /^\/the-omega-institute\/trureturing\/blob\/[a-f0-9]{40}\/(.+)\.lean$/.exec(
      url.pathname,
    );
    if (
      url.protocol !== 'https:' ||
      url.hostname !== 'github.com' ||
      url.username ||
      url.password ||
      !match
    )
      return;
    const module = decodeURIComponent(match[1]!);
    if (module.split('/').some((part) => !part || part === '..')) return;
    return `module:${module}`;
  } catch {
    return;
  }
}
