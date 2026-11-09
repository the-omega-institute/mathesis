import { useCallback, useId, useMemo, useRef, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { t, useLocale } from '../i18n/locale';
import { Link } from '../routing/router';
import { InlineLatex, LatexText, MacrosProvider } from './Markdown';
import { Async } from './ui';
import { EntrantName } from '../pages/Leaderboard';
import { ProofAttribution } from './ProofAttribution';
import { safeHttpUrl } from '../lib/links';
import {
  addCheckedUses,
  cleanSnapshot,
  dependencyLayout,
  dependencyAtlasLayout,
  loadDependencyGraph,
  loadFocusedDependencyGraph,
  resultDegree,
  verifiedKnowledge,
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

export function DependencyGraph({
  area,
  initialScope = 'all',
  focus,
}: {
  area?: string;
  initialScope?: 'verified' | 'all' | 'pending';
  focus?: string;
}) {
  const api = useApi();
  const [pages, setPages] = useState(2);
  const [scope, setScope] = useState(initialScope);
  const exact = /^statement:([^:]+):([1-9]\d*):([^:]+)$/.exec(focus ?? '');
  const focusWork = exact?.[1];
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
      if (focusWork && !graph.nodes.some((n) => n.id === focus)) {
        const focused = await loadFocusedDependencyGraph(api, focus!, signal);
        graph = cleanSnapshot({
          nodes: [...graph.nodes, ...focused.nodes],
          edges: [...graph.edges, ...focused.edges],
          next: graph.next,
        });
      }
      return graph;
    },
    [api, pages, focus, focusWork],
  );
  const data = useAsync(load);
  return (
    <Async state={data.state} onRetry={data.reload}>
      {(graph) => (
        <>
          <div className="formalization-key" aria-label={t('Node meanings')}>
            <span>
              <i className="formalization-mark is-verified" aria-hidden="true" />
              {t('Formalized in Lean')}
            </span>
            <span>
              <i className="formalization-mark" aria-hidden="true" />
              {t('Awaiting a formal proof')}
            </span>
            {graph.nodes.some((n) => n.disproved) ? (
              <span>
                <i className="formalization-mark is-disproved" aria-hidden="true" />
                {t('Disproved in Lean')}
              </span>
            ) : null}
          </div>
          <div
            role="group"
            aria-label={t('Knowledge and formalization')}
            className="dependency-toolbar dependency-scope"
          >
            {(['all', 'pending', 'verified'] as const).map((value) => (
              <button
                key={value}
                type="button"
                className="text-button"
                aria-pressed={scope === value}
                onClick={() => setScope(value)}
              >
                {t(
                  value === 'all'
                    ? 'All results'
                    : value === 'pending'
                      ? 'Awaiting formalization'
                      : 'Formalized in Lean',
                )}
                <span className="scope-count">
                  {
                    graph.nodes.filter(
                      (n) =>
                        n.kind === 'result' &&
                        (value === 'all' ||
                          (value === 'verified'
                            ? n.verified || n.disproved
                            : !n.verified && !n.disproved)),
                    ).length
                  }
                </span>
              </button>
            ))}
          </div>
          <DependencyGraphView
            key={scope}
            graph={
              scope === 'verified'
                ? verifiedKnowledge(graph)
                : scope === 'pending'
                  ? cleanSnapshot({
                      ...graph,
                      nodes: graph.nodes.filter(
                        (n) => n.kind !== 'result' || (!n.verified && !n.disproved),
                      ),
                    })
                  : graph
            }
            initialArea={area}
            focus={focus}
            empty={
              scope === 'verified'
                ? 'No independently verified formalizations yet. Explore the open results and contribute a proof.'
                : undefined
            }
          />
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

function inArea(node: DependencyNode, area: string) {
  return (
    !area ||
    node.areas.includes(area) ||
    (area === 'unclassified' && node.kind !== 'literature' && !node.areas.length)
  );
}

function initialResult(graph: DependencySnapshot, area: string) {
  return graph.nodes
    .filter((node) => node.kind === 'result' && inArea(node, area))
    .sort(
      (a, b) => resultDegree(graph, b.id) - resultDegree(graph, a.id) || a.id.localeCompare(b.id),
    )[0];
}

export function DependencyGraphView({
  graph: source,
  initialArea = '',
  focus,
  empty,
}: {
  graph: DependencySnapshot;
  initialArea?: string;
  focus?: string;
  empty?: string;
}) {
  const api = useApi();
  const locale = useLocale();
  const [area, setArea] = useState(initialArea === 'all' ? '' : initialArea);
  const [chosen, setChosen] = useState<string | undefined>(focus);
  const [proofOwner, setProofOwner] = useState<string>();
  const [overview, setOverview] = useState(!focus);
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
    (chosen && !overview ? undefined : initialResult(source, area));
  const usesWork = selection?.work;
  const usesClaim = selection?.claim;
  const loadUses = useCallback(
    (signal: AbortSignal) => api.resultUses(usesWork!, usesClaim!, { signal }),
    [api, usesWork, usesClaim],
  );
  const uses = useAsync(
    selection?.kind === 'result' && (selection.verified || selection.disproved) ? loadUses : null,
  );
  const graph = useMemo(() => addCheckedUses(source, uses.value ?? []), [source, uses.value]);
  const selected =
    graph.nodes.find((node) => node.id === chosen) ??
    (chosen && !overview ? undefined : initialResult(graph, area));
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  const resultNodes = graph.nodes.filter((node) => node.kind === 'result' && inArea(node, area));
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
    graph.nodes.filter((node) => inArea(node, area)).map((node) => node.id),
  );
  // Keep sources and checked prerequisites visible across a subject boundary.
  for (const edge of graph.edges) if (activeIds.has(edge.to)) activeIds.add(edge.from);
  const atlas = overview && !area;
  const visibleNodes = graph.nodes.filter((node) =>
    atlas
      ? node.kind === 'result' || node.kind === 'contribution'
      : overview
        ? activeIds.has(node.id)
        : localIds.has(node.id),
  );
  const ids = new Set(visibleNodes.map((node) => node.id));
  const visibleEdges = graph.edges.filter(
    (edge) =>
      ids.has(edge.from) &&
      ids.has(edge.to) &&
      (overview || edge.from === selected?.id || edge.to === selected?.id),
  );
  const overviewLayout = dependencyLayout({ ...graph, nodes: visibleNodes, edges: visibleEdges });
  const atlasLayout = dependencyAtlasLayout(
    { ...graph, nodes: visibleNodes, edges: visibleEdges },
    availableWidth,
  );
  const points = atlas
    ? atlasLayout.points
    : overview
      ? overviewLayout.points
      : new Map<string, { x: number; y: number }>();
  if (!overview && selected) {
    points.set(selected.id, {
      x: compact ? Math.max(20, (availableWidth - 240) / 2) : 360,
      y: compact ? 200 : 170,
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
              ? 170
              : 50 + index * (300 / Math.max(1, nodes.length - 1)),
        }),
      );
    }
  }
  const width = atlas
    ? atlasLayout.width
    : overview
      ? overviewLayout.width
      : compact
        ? availableWidth
        : 970;
  const height = atlas
    ? atlasLayout.height
    : overview
      ? overviewLayout.height
      : compact
        ? 480
        : 420;
  const fit = atlas
    ? 1
    : overview
      ? Math.max(0.6, Math.min(1, availableWidth / width, 600 / height))
      : compact
        ? 1
        : Math.min(1, availableWidth / 970);
  const overviewFocus = selected && points.get(selected.id);
  const centeredX =
    !atlas && overview && overviewFocus
      ? availableWidth / 2 - (overviewFocus.x + 120) * fit
      : (availableWidth - width * fit) / 2;
  const centeredY = !atlas && overview && overviewFocus ? 260 - overviewFocus.y * fit : 0;
  const scale = fit * camera.scale;
  const choose = (id: string) => {
    if (atlas) setArea(byId.get(id)?.areas[0] ?? 'unclassified');
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
    setOverview(true);
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
            atlas
              ? 'Explore the results and connections across mathematical fields. Select a field or a result to read further.'
              : 'Follow the results behind an argument, see what builds on them, and find the next contribution.',
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
            {!atlas ? (
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
            ) : null}
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
            <figure
              className={`dependency-figure${atlas ? ' is-atlas' : ''}`}
              aria-label={t('Mathematical dependency graph')}
            >
              <div
                className="dependency-viewport"
                style={atlas || !overview ? { height } : undefined}
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
                  className={`dependency-canvas${atlas ? ' is-atlas' : ''}${overview && scale < 0.4 ? ' is-distant' : ''}`}
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
                            atlas
                              ? `M ${a.x + 4} ${a.y + 4} L ${b.x + 4} ${b.y + 4}`
                              : compact && !overview
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
                  {atlas
                    ? atlasLayout.fields.map((field) => (
                        <button
                          type="button"
                          key={field.id}
                          className="dependency-atlas-field"
                          style={{ left: field.x, top: field.y, width: field.width }}
                          onClick={() => changeArea(field.id)}
                        >
                          <span>
                            {findTheoryArea(field.id)
                              ? localize(findTheoryArea(field.id)!.title, locale)
                              : t('Other mathematics')}
                          </span>
                          <small>{t('{count} results', { count: field.count })}</small>
                        </button>
                      ))
                    : null}
                  {visibleNodes.map((node) => {
                    const position = points.get(node.id)!;
                    return (
                      <MacrosProvider key={node.id} macros={node.macros}>
                        <button
                          type="button"
                          title={`${label(node)} — ${node.authors.join('; ')}`}
                          className={`dependency-node is-${node.kind}${!atlas && node.id === selected.id ? ' is-selected' : ''}${node.verified || node.disproved ? ' is-verified' : ''}${node.disproved ? ' is-disproved' : ''}`}
                          style={{ left: position.x, top: position.y }}
                          aria-label={`${label(node)} — ${node.authors.join('; ')}${node.kind === 'result' || node.kind === 'contribution' ? ` — ${t(node.disproved ? 'Disproved in Lean' : node.verified ? 'Formalized in Lean' : 'Awaiting a formal proof')}` : ''}`}
                          data-verification={
                            node.disproved ? 'disproved' : node.verified ? 'verified' : 'pending'
                          }
                          aria-pressed={!atlas && node.id === selected.id}
                          aria-controls={!atlas ? panel : undefined}
                          onClick={() => choose(node.id)}
                        >
                          <span className="dependency-dot" aria-hidden="true" />
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
                  {atlas
                    ? t(
                        'Each point is a result. Select a field or a point to explore its connections.',
                      )
                    : t('Drag to explore; select a node to read.')}
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
            {!atlas ? (
              <section
                className="dependency-detail"
                role="region"
                aria-label={t('Selected result and its dependencies')}
                id={panel}
              >
                <MacrosProvider macros={selected.macros}>
                  <p className="dependency-kicker">
                    {selected.disproved
                      ? t('Disproved in Lean')
                      : selected.verified
                        ? t('Lean checked')
                        : selected.kind === 'literature'
                          ? t('Earlier literature')
                          : (selected.label ?? t('Research'))}
                  </p>
                  <h2>
                    <InlineLatex source={selected.title} />
                  </h2>
                  <p className="dependency-credit">
                    {selected.authors.map((author, index) => (
                      <span key={`${author}:${index}`}>
                        {index ? '; ' : ''}
                        <Link to={{ kind: 'papers', author }}>{author}</Link>
                      </span>
                    ))}
                  </p>
                  {selected.contributor ? (
                    <p>
                      {t('Formalization: ')}
                      <EntrantName entrant={selected.contributor} />
                    </p>
                  ) : null}
                  <ProofAttribution attribution={selected.attribution ?? undefined} />
                  {selected.statement ? <LatexText source={selected.statement} /> : null}
                  {selected.kind === 'result' && !selected.verified && !selected.disproved ? (
                    <p className="formalization-invitation">
                      {t(
                        selected.target
                          ? 'This result is ready for a Lean proof. Your verified contribution will be credited here.'
                          : 'This result is awaiting a formal proof. Read the argument, build on its prerequisites, and share your Lean work. A matching Lean statement is needed for verification.',
                      )}
                    </p>
                  ) : null}

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
                        to={{ kind: 'paper', record: selected.work, version: selected.version }}
                      >
                        {sourcePaper ? (
                          <InlineLatex source={sourcePaper.title} />
                        ) : (
                          t('Read the paper')
                        )}
                      </Link>
                    ) : null}
                    {selected.work && selected.claim ? (
                      <Link
                        to={{
                          kind: 'result',
                          record: selected.work,
                          claim: selected.claim,
                          version: selected.collected ? selected.version : undefined,
                        }}
                      >
                        {t('Read result')}
                      </Link>
                    ) : null}
                    {selected.url && safeHttpUrl(selected.url) ? (
                      <a href={safeHttpUrl(selected.url)!}>{t('Original source')}</a>
                    ) : null}
                    {lean ? <a href={lean}>{t('Published Lean source for this paper')}</a> : null}
                    {selected.target ? (
                      <Link
                        to={{
                          kind: 'submit',
                          intake: 'lean',
                          record: selected.work!,
                          claim: selected.claim!,
                          version: selected.collected ? selected.version : undefined,
                        }}
                      >
                        {t(
                          selected.verified || selected.disproved
                            ? 'Add another proof'
                            : 'Contribute a formal proof',
                        )}
                      </Link>
                    ) : selected.kind === 'result' && !selected.verified && !selected.disproved ? (
                      <Link
                        to={{
                          kind: 'submit',
                          intake: 'lean',
                          record: selected.work,
                          claim: selected.claim,
                          version: selected.collected ? selected.version : undefined,
                        }}
                      >
                        {t('Contribute Lean work')}
                      </Link>
                    ) : null}
                    {selected.kind === 'result' ? (
                      <Link to={{ kind: 'formalizations', focus: selected.id }}>
                        {t('Link to this result')}
                      </Link>
                    ) : null}
                    {selected.proof && selected.work && selected.claim ? (
                      <a
                        href={`/api/v1/papers/${encodeURIComponent(selected.work)}/statements/${encodeURIComponent(selected.claim)}/attempts/${encodeURIComponent(selected.proof.id)}/bundle`}
                      >
                        {t('Read the checked proof and its sources')}
                      </a>
                    ) : null}
                  </p>
                  {[
                    {
                      label: 'Builds on',
                      edges: incoming.filter((e) =>
                        ['checked_use', 'declared_dependency'].includes(e.relation),
                      ),
                    },
                    {
                      label: 'Earlier references',
                      edges: incoming.filter((e) =>
                        ['manuscript_reference', 'cites'].includes(e.relation),
                      ),
                    },
                    {
                      label: 'A stepping stone for',
                      edges: outgoing.filter((e) =>
                        ['checked_use', 'declared_dependency'].includes(e.relation),
                      ),
                    },
                    {
                      label: 'Referenced by',
                      edges: outgoing.filter((e) =>
                        ['manuscript_reference', 'cites'].includes(e.relation),
                      ),
                    },
                    {
                      label: 'Results in this paper',
                      edges: outgoing.filter((e) => e.relation === 'contains'),
                    },
                  ]
                    .filter((group) => group.edges.length)
                    .map((group) => (
                      <div className="dependency-connections" key={group.label}>
                        <h3>{t(group.label)}</h3>
                        <ul className="dependency-reading-links">
                          {group.edges.map((edge) => {
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
                      </div>
                    ))}
                  {uses.state.status === 'error' ? (
                    <p>
                      <button type="button" className="text-button" onClick={uses.reload}>
                        {t('Retry loading checked proof dependencies')}
                      </button>
                    </p>
                  ) : null}
                </MacrosProvider>
              </section>
            ) : null}
            {!atlas ? (
              <nav
                className="dependency-result-navigation"
                aria-label={t('Explore further results')}
              >
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
                <Link to={{ kind: 'formalizations', view: 'list' }}>
                  {t('Find work awaiting formalization')}
                </Link>
              </nav>
            ) : null}
          </>
        ) : (
          <p>
            {t(
              chosen
                ? 'This result is not available in the current public graph.'
                : (empty ??
                    'No indexed results in this field have been loaded yet. Load more research or browse its existing theory.'),
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
