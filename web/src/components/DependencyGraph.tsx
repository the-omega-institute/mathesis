import { useCallback, useId, useMemo, useRef, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { t, useLocale } from '../i18n/locale';
import { Link } from '../routing/router';
import { InlineLatex, LatexText, MacrosProvider } from './Markdown';
import { Async } from './ui';
import { EntrantName } from '../pages/Leaderboard';
import { safeHttpUrl } from '../lib/links';
import {
  addCheckedUses,
  cleanSnapshot,
  dependencyLayout,
  loadDependencyGraph,
  resultDegree,
  type DependencyNode,
  type DependencyRelation,
  type DependencySnapshot,
} from '../lib/dependencyGraph';
import { findTheoryArea, localize, theoryAreas } from '../lib/theoryLibrary';
import '../styles/dependency-graph.css';

const relationLabel: Record<DependencyRelation, string> = {
  checked_use: 'Used in a checked Lean proof',
  declared_dependency: 'Dependency stated by the author',
  manuscript_reference: 'Reference in the mathematical argument',
  cites: 'Cited literature',
  contains: 'Result in this paper',
};

export function DependencyGraph({ area }: { area?: string }) {
  const api = useApi();
  const [pages, setPages] = useState(1);
  const load = useCallback(
    async (signal: AbortSignal) => {
      let graph: DependencySnapshot = { nodes: [], edges: [], next: null };
      let before: string | undefined;
      // Re-read previous pages too: a later expansion must respect current visibility.
      for (let page = 0; page < pages; page++) {
        const next = await loadDependencyGraph(api, before, signal);
        graph = cleanSnapshot({
          nodes: [...graph.nodes, ...next.nodes],
          edges: [...graph.edges, ...next.edges],
          next: next.next,
        });
        if (!next.next) break;
        before = next.next;
      }
      return graph;
    },
    [api, pages],
  );
  const data = useAsync(load);
  return (
    <Async state={data.state} onRetry={data.reload}>
      {(graph) => (
        <>
          <DependencyGraphView graph={graph} initialArea={area} />
          {graph.next ? (
            <p className="dependency-more">
              <button
                className="text-button"
                type="button"
                onClick={() => setPages((count) => count + 1)}
              >
                {t('Load more research into the graph')}
              </button>
            </p>
          ) : null}
        </>
      )}
    </Async>
  );
}

function initialResult(graph: DependencySnapshot, area: string) {
  return graph.nodes
    .filter((node) => node.kind === 'result' && (!area || node.areas.includes(area)))
    .sort(
      (a, b) => resultDegree(graph, b.id) - resultDegree(graph, a.id) || a.id.localeCompare(b.id),
    )[0];
}

export function DependencyGraphView({
  graph: source,
  initialArea = '',
}: {
  graph: DependencySnapshot;
  initialArea?: string;
}) {
  const api = useApi();
  const locale = useLocale();
  const [area, setArea] = useState(initialArea);
  const [chosen, setChosen] = useState<string>();
  const [proofOwner, setProofOwner] = useState<string>();
  const [overview, setOverview] = useState(false);
  const [connectionPage, setConnectionPage] = useState(0);
  const [camera, setCamera] = useState({ x: 0, y: 0, scale: 1 });
  const [availableWidth, setAvailableWidth] = useState(970);
  const measureViewport = useCallback((element: HTMLDivElement | null) => {
    if (!element) return;
    setAvailableWidth(element.clientWidth || 970);
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry) setAvailableWidth(entry.contentRect.width || 970);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const drag = useRef<{ x: number; y: number; originX: number; originY: number } | null>(null);
  const arrow = useId();
  const panel = useId();
  const selection =
    source.nodes.find((node) => node.id === chosen) ??
    source.nodes.find((node) => node.id === proofOwner) ??
    initialResult(source, area);
  const usesWork = selection?.work;
  const usesClaim = selection?.claim;
  const loadUses = useCallback(
    (signal: AbortSignal) => api.resultUses(usesWork!, usesClaim!, { signal }),
    [api, usesWork, usesClaim],
  );
  const uses = useAsync(
    selection?.kind === 'result' && selection.verified && !selection.collected ? loadUses : null,
  );
  const graph = useMemo(() => addCheckedUses(source, uses.value ?? []), [source, uses.value]);
  const selected = graph.nodes.find((node) => node.id === chosen) ?? initialResult(graph, area);
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  const resultNodes = graph.nodes.filter(
    (node) => node.kind === 'result' && (!area || node.areas.includes(area)),
  );
  const related = graph.edges.filter(
    (edge) =>
      (edge.relation !== 'contains' || selected?.kind === 'paper') &&
      (edge.from === selected?.id || edge.to === selected?.id),
  );
  const incoming = related.filter((edge) => edge.to === selected?.id);
  const outgoing = related.filter((edge) => edge.from === selected?.id);
  const compact = availableWidth < 640;
  const perPage = compact ? 1 : 3;
  const pages = Math.max(1, Math.ceil(Math.max(incoming.length, outgoing.length) / perPage));
  const offset = Math.min(connectionPage, pages - 1) * perPage;
  const neighbors = [
    ...incoming.slice(offset, offset + perPage),
    ...outgoing.slice(offset, offset + perPage),
  ];
  const localIds = new Set([selected?.id, ...neighbors.flatMap((edge) => [edge.from, edge.to])]);
  // An isolated result retains a visible source, without pretending membership is proof use.
  if (localIds.size === 1 && selected?.paper) localIds.add(selected.paper);
  const activeIds = new Set(
    graph.nodes.filter((node) => !area || node.areas.includes(area)).map((node) => node.id),
  );
  // Keep sources and checked prerequisites visible across a subject boundary.
  for (const edge of graph.edges) if (activeIds.has(edge.to)) activeIds.add(edge.from);
  const visibleNodes = graph.nodes.filter((node) =>
    overview ? activeIds.has(node.id) : localIds.has(node.id),
  );
  const ids = new Set(visibleNodes.map((node) => node.id));
  const visibleEdges = graph.edges.filter(
    (edge) =>
      ids.has(edge.from) &&
      ids.has(edge.to) &&
      (overview || edge.from === selected?.id || edge.to === selected?.id),
  );
  const overviewLayout = dependencyLayout({ ...graph, nodes: visibleNodes, edges: visibleEdges });
  const points = overview ? overviewLayout.points : new Map<string, { x: number; y: number }>();
  if (!overview && selected) {
    points.set(selected.id, {
      x: compact ? Math.max(20, (availableWidth - 240) / 2) : 360,
      y: compact ? 200 : 260,
    });
    const left = visibleNodes.filter(
      (node) =>
        node.id !== selected.id &&
        (incoming.some((edge) => edge.from === node.id) || node.id === selected.paper),
    );
    const leftIds = new Set(left.map((node) => node.id));
    const right = visibleNodes.filter((node) => node.id !== selected.id && !leftIds.has(node.id));
    for (const [nodes, x] of [
      [left, 30],
      [right, 690],
    ] as const) {
      nodes.forEach((node, index) =>
        points.set(node.id, {
          x: compact ? Math.max(20, (availableWidth - 240) / 2) : x,
          y: compact
            ? x === 30
              ? 40
              : 360
            : nodes.length === 1
              ? 260
              : 70 + index * (380 / Math.max(1, nodes.length - 1)),
        }),
      );
    }
  }
  const width = overview ? overviewLayout.width : compact ? availableWidth : 970;
  const height = overview ? overviewLayout.height : compact ? 480 : 600;
  const fit = overview
    ? Math.max(0.6, Math.min(1, availableWidth / width, 600 / height))
    : compact
      ? 1
      : Math.min(1, availableWidth / 970);
  const overviewFocus = selected && points.get(selected.id);
  const centeredX =
    overview && overviewFocus
      ? availableWidth / 2 - (overviewFocus.x + 120) * fit
      : (availableWidth - width * fit) / 2;
  const centeredY = overview && overviewFocus ? 260 - overviewFocus.y * fit : 0;
  const scale = fit * camera.scale;
  const choose = (id: string) => {
    if (selected?.kind === 'result') setProofOwner(selected.id);
    setChosen(id);
    setConnectionPage(0);
    setOverview(false);
    setCamera({ x: 0, y: 0, scale: 1 });
  };
  const changeArea = (id: string) => {
    setArea(id);
    setChosen(undefined);
    setProofOwner(undefined);
    setConnectionPage(0);
    setOverview(false);
    setCamera({ x: 0, y: 0, scale: 1 });
  };
  const label = (node: DependencyNode) =>
    node.label && node.label !== node.title ? `${node.label}: ${node.title}` : node.title;
  const sourcePaper = selected?.paper ? byId.get(selected.paper) : undefined;
  const lean = sourcePaper?.leanSource && safeHttpUrl(sourcePaper.leanSource);
  return (
    <div className="dependency-reading">
      <nav className="dependency-fields" aria-label={t('Fields of mathematics')}>
        <select
          className="dependency-field-select"
          aria-label={t('Field of mathematics')}
          value={area}
          onChange={(event) => changeArea(event.target.value)}
        >
          <option value="">{t('All fields')}</option>
          {theoryAreas.map((field) => (
            <option key={field.id} value={field.id}>
              {localize(field.title, locale)}
            </option>
          ))}
        </select>
        <button
          type="button"
          className="text-button"
          aria-current={!area ? 'page' : undefined}
          onClick={() => changeArea('')}
        >
          {t('All fields')}
        </button>
        {theoryAreas.map((field) => (
          <button
            key={field.id}
            type="button"
            className="text-button"
            aria-current={field.id === area ? 'page' : undefined}
            onClick={() => changeArea(field.id)}
          >
            {localize(field.title, locale)}
          </button>
        ))}
        <Link
          className="dependency-theory-index"
          to={{ kind: 'research_graph', focus: `library:${area || 'all'}` }}
        >
          {t('Browse existing theory')}
        </Link>
      </nav>
      <div className="dependency-content">
        <p className="dependency-intro">
          {t(
            'Follow the results behind an argument, see what builds on them, and find the next contribution.',
          )}
        </p>
        <div className="dependency-toolbar">
          <p>
            {area && findTheoryArea(area)
              ? localize(findTheoryArea(area)!.title, locale)
              : t('Mathematical knowledge')}{' '}
            <span>· {t('{count} results loaded', { count: resultNodes.length })}</span>
          </p>
          <div role="group" aria-label={t('Graph view')}>
            <button
              type="button"
              className="text-button"
              aria-pressed={overview}
              onClick={() => {
                setOverview(!overview);
                setCamera({ x: 0, y: 0, scale: 1 });
              }}
            >
              {overview ? t('Focus on the selected result') : t('Explore the full graph')}
            </button>
            <button
              type="button"
              className="text-button"
              aria-label={t('Zoom out')}
              onClick={() => setCamera((v) => ({ ...v, scale: Math.max(0.25, v.scale / 1.25) }))}
            >
              −
            </button>
            <button
              type="button"
              className="text-button"
              aria-label={t('Zoom in')}
              onClick={() => setCamera((v) => ({ ...v, scale: Math.min(10, v.scale * 1.25) }))}
            >
              +
            </button>
            <button
              type="button"
              className="text-button"
              onClick={() => setCamera({ x: 0, y: 0, scale: 1 })}
            >
              {t('Reset view')}
            </button>
          </div>
        </div>
        {selected ? (
          <>
            <figure className="dependency-figure" aria-label={t('Mathematical dependency graph')}>
              <div
                className="dependency-viewport"
                ref={measureViewport}
                role="region"
                tabIndex={0}
                aria-label={t('Drag or use the arrow keys to move through the graph')}
                onKeyDown={(event) => {
                  if (
                    event.target !== event.currentTarget ||
                    !['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)
                  )
                    return;
                  event.preventDefault();
                  setCamera((v) => ({
                    ...v,
                    x:
                      v.x + (event.key === 'ArrowLeft' ? 80 : event.key === 'ArrowRight' ? -80 : 0),
                    y: v.y + (event.key === 'ArrowUp' ? 80 : event.key === 'ArrowDown' ? -80 : 0),
                  }));
                }}
                onPointerDown={(event) => {
                  if ((event.target as HTMLElement).closest('button')) return;
                  drag.current = {
                    x: event.clientX,
                    y: event.clientY,
                    originX: camera.x,
                    originY: camera.y,
                  };
                  event.currentTarget.setPointerCapture(event.pointerId);
                }}
                onPointerMove={(event) => {
                  const value = drag.current;
                  if (value)
                    setCamera((v) => ({
                      ...v,
                      x: value.originX + event.clientX - value.x,
                      y: value.originY + event.clientY - value.y,
                    }));
                }}
                onPointerUp={() => {
                  drag.current = null;
                }}
                onPointerCancel={() => {
                  drag.current = null;
                }}
              >
                <div
                  className={`dependency-canvas${overview && scale < 0.4 ? ' is-distant' : ''}`}
                  style={{
                    width,
                    height,
                    transform: `translate(${camera.x + centeredX}px,${camera.y + centeredY}px) scale(${scale})`,
                  }}
                >
                  <svg width={width} height={height} aria-hidden="true">
                    <defs>
                      <marker
                        id={arrow}
                        viewBox="0 0 10 10"
                        refX="9"
                        refY="5"
                        markerWidth="5"
                        markerHeight="5"
                        orient="auto"
                      >
                        <path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
                      </marker>
                    </defs>
                    {visibleEdges.map((edge) => {
                      const a = points.get(edge.from)!;
                      const b = points.get(edge.to)!;
                      return (
                        <path
                          key={JSON.stringify(edge)}
                          className={`dependency-edge is-${edge.relation}${edge.from === selected.id || edge.to === selected.id ? ' is-active' : ''}`}
                          data-relation={edge.relation}
                          d={
                            compact && !overview
                              ? `M ${a.x + 3} ${a.y + 25} C ${a.x - 40} ${a.y + 65}, ${b.x - 40} ${b.y - 15}, ${b.x - 8} ${b.y + 20}`
                              : b.x > a.x
                                ? `M ${a.x + 245} ${a.y + 20} C ${a.x + 280} ${a.y + 20}, ${b.x - 45} ${b.y + 20}, ${b.x - 8} ${b.y + 20}`
                                : `M ${a.x - 8} ${a.y + 36} C ${a.x - 45} ${a.y + 36}, ${b.x + 280} ${b.y + 36}, ${b.x + 245} ${b.y + 36}`
                          }
                          markerEnd={edge.relation === 'contains' ? undefined : `url(#${arrow})`}
                        />
                      );
                    })}
                  </svg>
                  {visibleNodes.map((node) => {
                    const position = points.get(node.id)!;
                    return (
                      <MacrosProvider key={node.id} macros={node.macros}>
                        <button
                          type="button"
                          title={`${label(node)} — ${node.authors.join('; ')}`}
                          className={`dependency-node is-${node.kind}${node.id === selected.id ? ' is-selected' : ''}${node.verified ? ' is-verified' : ''}`}
                          style={{ left: position.x, top: position.y }}
                          aria-label={`${label(node)} — ${node.authors.join('; ')}`}
                          aria-pressed={node.id === selected.id}
                          aria-controls={panel}
                          onClick={() => choose(node.id)}
                        >
                          <span className="dependency-dot" />
                          <span className="dependency-node-label">
                            <InlineLatex source={label(node)} />
                          </span>
                          <span className="dependency-node-authors">{node.authors.join('; ')}</span>
                        </button>
                      </MacrosProvider>
                    );
                  })}
                </div>
              </div>
              <figcaption className="dependency-legend">
                {(
                  [
                    'checked_use',
                    'declared_dependency',
                    'manuscript_reference',
                    'cites',
                    'contains',
                  ] as const
                )
                  .filter((relation) => visibleEdges.some((edge) => edge.relation === relation))
                  .map((relation) => (
                    <span key={relation} className={`is-${relation}`}>
                      {t(relationLabel[relation])}
                    </span>
                  ))}
                <span className="dependency-map-help">
                  {t('Drag to explore; select a node to read.')}
                </span>
              </figcaption>
            </figure>
            {pages > 1 && !overview ? (
              <p className="dependency-pagination">
                <button
                  className="text-button"
                  type="button"
                  disabled={!connectionPage}
                  onClick={() => setConnectionPage((p) => p - 1)}
                >
                  {t('Previous connections')}
                </button>
                <button
                  className="text-button"
                  type="button"
                  disabled={connectionPage >= pages - 1}
                  onClick={() => setConnectionPage((p) => p + 1)}
                >
                  {t('More connections')}
                </button>
              </p>
            ) : null}
            <section
              className="dependency-detail"
              role="region"
              aria-label={t('Selected result and its dependencies')}
              id={panel}
            >
              <MacrosProvider macros={selected.macros}>
                <p className="dependency-kicker">
                  {selected.verified
                    ? t('Lean checked')
                    : selected.kind === 'literature'
                      ? t('Earlier literature')
                      : (selected.label ?? t('Research'))}
                </p>
                <h2>
                  <InlineLatex source={selected.title} />
                </h2>
                <p className="dependency-credit">{selected.authors.join('; ')}</p>
                {selected.contributor ? (
                  <p>
                    {t('Formalization: ')}
                    <EntrantName entrant={selected.contributor} />
                  </p>
                ) : null}
                {selected.statement ? <LatexText source={selected.statement} /> : null}
                <p className="dependency-detail-links">
                  {sourcePaper ? (
                    <button
                      type="button"
                      className="text-button"
                      onClick={() => choose(sourcePaper.id)}
                    >
                      {t('View this paper’s sources and results')}
                    </button>
                  ) : null}
                  {selected.work ? (
                    <Link
                      to={
                        selected.kind === 'contribution' || (!selected.collected && selected.claim)
                          ? { kind: 'result', record: selected.work, claim: selected.claim! }
                          : { kind: 'paper', record: selected.work, version: selected.version }
                      }
                    >
                      {sourcePaper ? (
                        <InlineLatex source={sourcePaper.title} />
                      ) : (
                        t('Read the paper')
                      )}
                    </Link>
                  ) : null}
                  {selected.url && safeHttpUrl(selected.url) ? (
                    <a href={safeHttpUrl(selected.url)!}>{t('Original source')}</a>
                  ) : null}
                  {lean ? <a href={lean}>{t('Published Lean source for this paper')}</a> : null}
                  {selected.target ? (
                    <Link to={{ kind: 'result', record: selected.work!, claim: selected.claim! }}>
                      {t('Contribute a formal proof')}
                    </Link>
                  ) : selected.kind === 'result' && !selected.verified ? (
                    <Link to={{ kind: 'submit', intake: 'lean' }}>{t('Contribute Lean work')}</Link>
                  ) : null}
                </p>
                {related.length ? (
                  <ul className="dependency-reading-links">
                    {related.map((edge) => {
                      const node = byId.get(edge.from === selected.id ? edge.to : edge.from)!;
                      return (
                        <li key={JSON.stringify(edge)}>
                          <button
                            type="button"
                            className="text-button"
                            onClick={() => choose(node.id)}
                          >
                            <InlineLatex source={label(node)} />
                          </button>
                          <span>{t(relationLabel[edge.relation])}</span>
                        </li>
                      );
                    })}
                  </ul>
                ) : null}
                {uses.state.status === 'error' ? (
                  <p>
                    <button type="button" className="text-button" onClick={uses.reload}>
                      {t('Retry loading checked proof dependencies')}
                    </button>
                  </p>
                ) : null}
              </MacrosProvider>
            </section>
            <nav className="dependency-result-navigation" aria-label={t('Explore further results')}>
              <button
                type="button"
                className="text-button"
                onClick={() => {
                  const index = resultNodes.findIndex((node) => node.id === selected.id);
                  const node = resultNodes[(index + 1) % resultNodes.length];
                  if (node) choose(node.id);
                }}
              >
                {t('Explore another result')}
              </button>
              <Link to={{ kind: 'formalizations' }}>{t('Find work awaiting formalization')}</Link>
            </nav>
          </>
        ) : (
          <p>
            {t(
              'No indexed results in this field have been loaded yet. Load more research or browse its existing theory.',
            )}
          </p>
        )}
        <details className="dependency-evidence-note">
          <summary>{t('How to read the connections')}</summary>
          <p>
            {t(
              'Solid arrows distinguish author-stated dependencies from uses inspected in a checked Lean proof. Dashed arrows record references in the manuscript; dotted connections to literature show citations. Paper membership and subject placement do not assert proof dependencies. A published Lean source is separate from a checked proof of this exact result.',
            )}
          </p>
        </details>
      </div>
    </div>
  );
}
