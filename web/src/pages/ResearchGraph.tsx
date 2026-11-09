import { t } from '../i18n/locale';
import { lazy, Suspense, useCallback, useId, useState } from 'react';
import { LeanKnowledgeGraph } from '../components/LeanKnowledgeGraph';
import { MathematicalKnowledge } from '../components/MathematicalKnowledge';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { ResearchGraph, ResearchNode } from '../api/types';
import { Async } from '../components/ui';
import { InlineLatex } from '../components/Markdown';
import { safeHttpUrl } from '../lib/links';
import { Link } from '../routing/router';

const DependencyGraph = lazy(() =>
  import('../components/DependencyGraph').then((module) => ({ default: module.DependencyGraph })),
);

const TheoryLibrary = lazy(() =>
  import('../components/TheoryLibrary').then((module) => ({ default: module.TheoryLibrary })),
);

export function ResearchGraphPage({
  focus,
  q,
  before,
  embedded = false,
}: {
  focus?: string;
  q?: string;
  before?: string;
  embedded?: boolean;
}) {
  const [layer, setLayer] = useState<'graph' | 'library' | 'results' | 'lean' | 'citations'>(
    focus?.startsWith('library:')
      ? 'library'
      : focus?.startsWith('theory:')
        ? 'graph'
        : focus?.startsWith('statement:')
          ? 'graph'
          : focus || q || before
            ? 'citations'
            : 'graph',
  );
  const api = useApi();
  const citationFocus = focus?.startsWith('statement:') ? `paper:${focus.split(':')[1]}` : focus;
  const load = useCallback(
    (signal: AbortSignal) => api.researchGraph({ focus: citationFocus, q, before }, { signal }),
    [api, citationFocus, q, before],
  );
  const graph = useAsync(layer === 'citations' ? load : null);
  const Heading = embedded ? 'h2' : 'h1';
  return (
    <div className={embedded ? 'research-graph' : 'page research-graph'}>
      {!embedded || layer !== 'graph' ? (
        <Heading>
          {t(
            layer === 'graph'
              ? 'Dependency graph'
              : layer === 'library'
                ? 'Theory resources'
                : layer === 'results'
                  ? 'Mathematical results'
                  : layer === 'lean'
                    ? 'Verified proof connections'
                    : 'Papers and sources',
          )}
        </Heading>
      ) : null}
      {layer !== 'library' && layer !== 'graph' ? (
        <>
          <p>
            <Link to={{ kind: 'research_graph' }}>{t('Back to the dependency graph')}</Link>
          </p>
          <div className="filter-row" role="group" aria-label={t('Knowledge graph layer')}>
            {(
              [
                ['results', 'Mathematical results'],
                ['lean', 'Verified proof connections'],
                ['citations', 'Papers and sources'],
              ] as const
            ).map(([value, label]) => (
              <button
                key={value}
                type="button"
                className="text-button"
                aria-pressed={layer === value}
                onClick={() => setLayer(value)}
              >
                {t(label)}
              </button>
            ))}
          </div>
        </>
      ) : null}
      {layer === 'graph' ? (
        <Suspense fallback={<p>{t('Loading…')}</p>}>
          <DependencyGraph
            area={focus?.startsWith('theory:') ? focus.slice(7) : undefined}
            focus={focus?.startsWith('statement:') ? focus : undefined}
          />
        </Suspense>
      ) : null}
      {layer === 'library' ? (
        <>
          <Suspense fallback={<p>{t('Loading…')}</p>}>
            <p>
              <Link to={{ kind: 'research_graph' }}>{t('Back to the dependency graph')}</Link>
            </p>
            <TheoryLibrary
              focus={
                focus?.startsWith('library:') && focus !== 'library:all'
                  ? 'theory:' + focus.slice(8)
                  : undefined
              }
            />
          </Suspense>
          {layer === 'library' ? (
            <p className="small">
              <button type="button" className="text-button" onClick={() => setLayer('lean')}>
                {t('Explore independently checked proof connections')}
              </button>
            </p>
          ) : null}
        </>
      ) : null}
      {layer === 'results' ? <MathematicalKnowledge focus={focus} /> : null}
      {layer === 'lean' ? <LeanKnowledgeGraph /> : null}
      {layer === 'citations' ? (
        <Async state={graph.state} onRetry={graph.reload}>
          {(value) => (
            <GraphView
              key={`${focus ?? ''}:${q ?? ''}:${before ?? ''}`}
              graph={value}
              focus={citationFocus}
              q={q}
            />
          )}
        </Async>
      ) : null}
    </div>
  );
}

const REFERENCES_PER_VIEW = 8;
const ROW_HEIGHT = 112;
type Citation = ResearchGraph['edges'][number];

/** Layout presents supplied citations; it does not infer a proof-dependency order. */
export function GraphView({
  graph,
  focus,
  q,
}: {
  graph: ResearchGraph;
  focus?: string;
  q?: string;
}) {
  const panelId = useId();
  const arrowId = useId();
  const nodes = new Map(graph.nodes.map((node) => [node.id, node]));
  const edges = graph.edges.filter((edge) => nodes.has(edge.from) && nodes.has(edge.to));
  const degree = new Map<string, number>();
  const pairs = new Map<string, Citation>();
  for (const edge of edges) {
    const key = JSON.stringify([edge.from, edge.to]);
    if (!pairs.has(key)) {
      degree.set(edge.to, (degree.get(edge.to) ?? 0) + 1);
      pairs.set(key, edge);
    }
  }
  const literature = graph.nodes
    .filter((n) => n.kind === 'literature')
    .sort(
      (a, b) => (degree.get(b.id) ?? 0) - (degree.get(a.id) ?? 0) || a.title.localeCompare(b.title),
    );
  const initial = nodes.get(focus ?? '') ?? literature[0] ?? graph.nodes[0];
  const [selection, setSelection] = useState(initial?.id);
  const [page, setPage] = useState(() =>
    Math.max(
      0,
      Math.floor(literature.findIndex((n) => n.id === initial?.id) / REFERENCES_PER_VIEW),
    ),
  );
  const selected = nodes.get(selection ?? '') ?? initial;
  const selectedReferenceIndex = literature.findIndex((n) => n.id === selected?.id);
  const currentPage =
    selectedReferenceIndex >= 0
      ? Math.floor(selectedReferenceIndex / REFERENCES_PER_VIEW)
      : Math.min(page, Math.max(0, Math.ceil(literature.length / REFERENCES_PER_VIEW) - 1));
  const earlier = literature.slice(
    currentPage * REFERENCES_PER_VIEW,
    (currentPage + 1) * REFERENCES_PER_VIEW,
  );
  const earlierIds = new Set(earlier.map((n) => n.id));
  const visibleEdges = [...pairs.values()].filter((e) => earlierIds.has(e.to));
  const citingIds = new Set(visibleEdges.map((e) => e.from));
  const papers = graph.nodes.filter(
    (n) =>
      n.kind === 'manuscript' &&
      (citingIds.has(n.id) || !edges.some((e) => e.from === n.id) || n.id === selected?.id),
  );
  const height = Math.max(360, Math.max(earlier.length, papers.length) * ROW_HEIGHT + 64);
  const positions = new Map<string, { x: number; y: number }>();
  for (const [column, items] of [earlier, papers].entries()) {
    items.forEach((node, index) =>
      positions.set(node.id, {
        x: column === 0 ? 16 : 460,
        y: 48 + (height - 64 - items.length * ROW_HEIGHT) / 2 + index * ROW_HEIGHT,
      }),
    );
  }
  const select = (id: string) => {
    setSelection(id);
    const index = literature.findIndex((n) => n.id === id);
    if (index >= 0) setPage(Math.floor(index / REFERENCES_PER_VIEW));
  };
  const turnPage = (next: number) => {
    setPage(next);
    const node = literature[next * REFERENCES_PER_VIEW];
    if (node) setSelection(node.id);
  };
  const selectedEdges = edges.filter((e) => e.from === selected?.id || e.to === selected?.id);
  const connected = new Set(selectedEdges.flatMap((e) => [e.from, e.to]));
  return (
    <>
      {selected ? (
        <div className="research-map-layout">
          <figure className="research-map" aria-label={t('Research connections')}>
            <div
              className="research-map-scroll"
              role="region"
              tabIndex={0}
              aria-label={t('Scrollable citation map')}
            >
              <div className="research-map-canvas" style={{ height }}>
                <span className="research-map-column" style={{ left: '2.2%' }}>
                  {t('Earlier literature')}
                </span>
                <span className="research-map-column" style={{ left: '63%' }}>
                  {t('Mathesis papers')}
                </span>
                <svg viewBox={`0 0 730 ${height}`} preserveAspectRatio="none" aria-hidden="true">
                  <defs>
                    <marker
                      id={arrowId}
                      viewBox="0 0 10 10"
                      refX="9"
                      refY="5"
                      markerWidth="6"
                      markerHeight="6"
                      orient="auto-start-reverse"
                    >
                      <path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
                    </marker>
                  </defs>
                  {visibleEdges.map((edge) => {
                    const a = positions.get(edge.to);
                    const b = positions.get(edge.from);
                    if (!a || !b) return null;
                    const active = edge.from === selected.id || edge.to === selected.id;
                    return (
                      <path
                        key={JSON.stringify([edge.from, edge.to])}
                        className={`research-map-edge${active ? ' is-active' : ''}`}
                        d={`M 274 ${a.y + 48} C 354 ${a.y + 48}, 370 ${b.y + 48}, 450 ${b.y + 48}`}
                        markerEnd={`url(#${arrowId})`}
                      />
                    );
                  })}
                </svg>
                {[...earlier, ...papers].map((node) => {
                  const position = positions.get(node.id)!;
                  return (
                    <button
                      key={node.id}
                      type="button"
                      className={`research-map-node ${node.kind}${connected.has(node.id) || selected.id === node.id ? ' is-connected' : ''}`}
                      style={{ left: `${(position.x / 730) * 100}%`, top: position.y }}
                      aria-pressed={selected.id === node.id}
                      aria-controls={panelId}
                      onClick={() => select(node.id)}
                      title={[node.title, ...node.authors].join(' · ')}
                    >
                      <span className="research-map-node-title">
                        <InlineLatex source={node.title} />
                      </span>{' '}
                      {node.authors.length ? (
                        <span className="research-map-node-authors">{node.authors.join('; ')}</span>
                      ) : null}
                    </button>
                  );
                })}
              </div>
            </div>
            <figcaption>
              {t(
                'Arrows lead from an earlier work to papers that cite it. Select a work to see its authors and connections.',
              )}
            </figcaption>
            {literature.length > REFERENCES_PER_VIEW ? (
              <nav className="research-map-paging" aria-label={t('References in this map')}>
                <button
                  type="button"
                  disabled={currentPage === 0}
                  onClick={() => turnPage(currentPage - 1)}
                >
                  {t('Previous references')}
                </button>
                <span>
                  {currentPage * REFERENCES_PER_VIEW + 1}–
                  {Math.min((currentPage + 1) * REFERENCES_PER_VIEW, literature.length)} {t(' of')}{' '}
                  {literature.length} {t(' references in this view')}
                </span>
                <button
                  type="button"
                  disabled={(currentPage + 1) * REFERENCES_PER_VIEW >= literature.length}
                  onClick={() => turnPage(currentPage + 1)}
                >
                  {t('Next references')}
                </button>
              </nav>
            ) : null}
          </figure>
          <WorkDetails
            key={selected.id}
            id={panelId}
            node={selected}
            nodes={nodes}
            edges={selectedEdges}
            onSelect={select}
          />
        </div>
      ) : (
        <div className="research-map-empty">
          <h2>{focus || q ? t('No works in this view') : t('No public connections yet')}</h2>
          <p>
            {focus || q
              ? t('No public work is available for this view.')
              : t('The map will grow as papers and their references become publicly available.')}
          </p>
        </div>
      )}
      {graph.next_before ? (
        <p>
          <Link to={{ kind: 'research_graph', focus, q, before: graph.next_before }}>
            {t('Next group of papers')}
          </Link>
        </p>
      ) : null}
      <details className="research-map-about">
        <summary>{t('About this graph')}</summary>
        <p>
          {t(
            'Connections show citations recorded in public manuscripts. Mathematical dependencies and checked formalizations have their own evidence on each result page.',
          )}
        </p>
        <p>{graph.scope}</p>
        <ul>
          {graph.coverage.map((c) => (
            <li key={c.record}>
              <Link to={{ kind: 'paper', record: c.record }}>{c.record}</Link> {t(' · version ')}
              {c.version}
              {!c.method
                ? t(' · references not indexed')
                : c.method === 'pdf_unindexed_v1'
                  ? t(' · PDF references not indexed')
                  : ''}
              <ul>
                {c.limitations.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      </details>
    </>
  );
}

function WorkDetails({
  id,
  node,
  nodes,
  edges,
  onSelect,
}: {
  id: string;
  node: ResearchNode;
  nodes: Map<string, ResearchNode>;
  edges: Citation[];
  onSelect: (id: string) => void;
}) {
  const href = node.url ? safeHttpUrl(node.url) : null;
  const neighbors = [...new Set(edges.map((e) => (e.from === node.id ? e.to : e.from)))]
    .map((id) => nodes.get(id))
    .filter((n): n is ResearchNode => Boolean(n));
  return (
    <section id={id} className="research-map-detail" aria-label={t('Selected work')}>
      <p className="entry-meta">
        {node.kind === 'literature' ? t('Earlier work') : t('Published paper')}
      </p>
      <h2>
        <InlineLatex source={node.title} />
      </h2>
      {node.authors.length ? (
        <p className="research-map-credit">{node.authors.join('; ')}</p>
      ) : null}
      <p className="research-map-links">
        <Link to={{ kind: 'research_graph', focus: node.id }}>
          {t('Explore these connections')}
        </Link>
        {node.record ? (
          <Link to={{ kind: 'paper', record: node.record, version: node.version?.toString() }}>
            {t('Read paper and results')}
          </Link>
        ) : null}
        {href ? (
          <a href={href} target="_blank" rel="noreferrer">
            {t('Original source')}
          </a>
        ) : null}
      </p>
      {neighbors.length ? (
        <>
          <h3>{node.kind === 'literature' ? t('Cited by') : t('References')}</h3>
          <ul className="research-map-neighbors">
            {neighbors.map((n) => (
              <li key={n.id}>
                <button type="button" onClick={() => onSelect(n.id)}>
                  <InlineLatex source={n.title} />
                </button>
              </li>
            ))}
          </ul>
        </>
      ) : (
        <p className="muted">
          {t('No citation connections are available for this work in this view.')}
        </p>
      )}
      {edges.length ? (
        <details>
          <summary>{t('Source evidence')}</summary>
          <ol className="research-map-evidence">
            {edges.map((edge) => (
              <li key={`${edge.record}:${edge.version}:${edge.reference_key}`}>
                <Link to={{ kind: 'paper', record: edge.record }}>
                  {nodes.get(edge.from)?.title ?? edge.record}
                </Link>
                <p>
                  {edge.title?.trim() ||
                    edge.display_text ||
                    nodes.get(edge.to)?.title ||
                    'Cited work'}
                </p>
                {edge.authors.length ? (
                  <p>
                    {edge.authors.join('; ')}
                    {edge.year ? ` (${edge.year})` : ''}
                  </p>
                ) : null}
                <pre className="reference-evidence">{edge.raw}</pre>
                <p className="small">
                  {t('Reference key ')}
                  {edge.reference_key} · {edge.source_location} {t(' · version')} {edge.version}
                  {edge.raw_truncated ? t(' · source excerpt') : ''}
                </p>
              </li>
            ))}
          </ol>
        </details>
      ) : null}
    </section>
  );
}
