import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useAsync } from '../api/useAsync';
import { useApi } from '../api/context';
import { listDirectoryNotes } from '../lib/paperDirectory';
import { sharedLibraryFocus } from '../lib/sharedLibrary';
import type { CollectedPaper } from '../api/types';
import { Link } from '../routing/router';
import { t } from '../i18n/locale';
import { loadSharedLibrary, type LibraryNode, type SharedLibrary } from '../lib/sharedLibrary';
import { Async, ExternalLink } from './ui';
import { LatexText } from './Markdown';

export function SharedLibraryGraph({ focus }: { focus?: string }) {
  const api = useApi();
  const papers = useAsync(
    useCallback((signal: AbortSignal) => listDirectoryNotes(api, {}, signal), [api]),
  );
  const library = useAsync(useCallback((signal: AbortSignal) => loadSharedLibrary(signal), []));
  return (
    <Async state={library.state} onRetry={library.reload}>
      {(data) => (
        <LibraryExplorer
          data={data}
          onRefresh={library.reload}
          initialFocus={focus}
          papers={papers.value?.items ?? []}
        />
      )}
    </Async>
  );
}
const HEIGHT = 600;
function frame(nodes: LibraryNode[], width: number) {
  let left = Infinity,
    right = -Infinity,
    top = Infinity,
    bottom = -Infinity;
  for (const node of nodes) {
    left = Math.min(left, node.x);
    right = Math.max(right, node.x);
    top = Math.min(top, node.y);
    bottom = Math.max(bottom, node.y);
  }
  return {
    x: (left + right) / 2 || 0,
    y: (top + bottom) / 2 || 0,
    scale: Math.min(
      (width - 50) / Math.max(right - left, 1),
      (HEIGHT - 50) / Math.max(bottom - top, 1),
    ),
  };
}
function LibraryExplorer({
  data,
  onRefresh,
  initialFocus,
  papers,
}: {
  data: SharedLibrary;
  onRefresh: () => void;
  initialFocus?: string;
  papers: CollectedPaper[];
}) {
  const [domain, setDomain] = useState('');
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(initialFocus ?? '');
  const [viewport, setViewport] = useState<{ zoom: number; x: number; y: number; anchor?: string }>(
    { zoom: initialFocus ? 5 : 1, x: 0, y: 0, anchor: initialFocus },
  );
  const [width, setWidth] = useState(900);
  const canvas = useRef<HTMLCanvasElement>(null);
  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const byId = useMemo(() => new Map(data.nodes.map((node) => [node.id, node])), [data]);
  const domains = useMemo(() => [...new Set(data.nodes.map((node) => node.domain))].sort(), [data]);
  const visible = useMemo(
    () => data.nodes.filter((node) => !domain || node.domain === domain),
    [data, domain],
  );
  const ids = useMemo(() => new Set(visible.map((node) => node.id)), [visible]);
  const edges = useMemo(
    () => data.edges.filter((edge) => ids.has(edge.from) && ids.has(edge.to)),
    [data, ids],
  );
  const geometry = useMemo(() => frame(visible, width), [visible, width]);
  const node = byId.get(selected);
  const linkedPapers = papers.filter(
    (paper) => sharedLibraryFocus(paper.lean?.url) === `module:${selected}`,
  );
  const related = useMemo(
    () =>
      new Set(
        data.edges
          .filter((edge) => edge.from === selected || edge.to === selected)
          .flatMap((edge) => [edge.from, edge.to]),
      ),
    [data, selected],
  );
  const matches = useMemo(
    () =>
      query.trim()
        ? data.nodes
            .filter((node) =>
              `${node.title} ${node.id}`.toLowerCase().includes(query.trim().toLowerCase()),
            )
            .slice(0, 12)
        : [],
    [data, query],
  );
  const formalized = data.nodes.filter((node) => node.formalized).length;

  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      const size = entries[0]?.contentRect.width;
      if (size) setWidth(size);
    });
    observer.observe(element);
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      setViewport((previous) => ({
        ...previous,
        zoom: Math.max(0.5, Math.min(100, previous.zoom * Math.exp(-event.deltaY * 0.002))),
      }));
    };
    element.addEventListener('wheel', wheel, { passive: false });
    return () => {
      observer.disconnect();
      element.removeEventListener('wheel', wheel);
    };
  }, []);
  useEffect(() => {
    const element = canvas.current,
      context = element?.getContext('2d');
    if (!element || !context) return;
    const dpr = window.devicePixelRatio || 1;
    element.width = width * dpr;
    element.height = HEIGHT * dpr;
    context.scale(dpr, dpr);
    const origin = byId.get(viewport.anchor ?? '') ?? geometry;
    const point = (node: LibraryNode) => ({
      x: width / 2 + viewport.x + (node.x - origin.x) * geometry.scale * viewport.zoom,
      y: HEIGHT / 2 + viewport.y + (node.y - origin.y) * geometry.scale * viewport.zoom,
    });
    const onScreen = (point: { x: number; y: number }) =>
      point.x >= -20 && point.x <= width + 20 && point.y >= -20 && point.y <= HEIGHT + 20;
    context.lineWidth = 0.5;
    context.strokeStyle = '#7e948926';
    context.beginPath();
    for (const edge of edges) {
      const from = point(byId.get(edge.from)!),
        to = point(byId.get(edge.to)!);
      if (!onScreen(from) && !onScreen(to)) continue;
      context.moveTo(from.x, from.y);
      context.lineTo(to.x, to.y);
    }
    context.stroke();
    if (selected) {
      context.beginPath();
      context.strokeStyle = '#9c6b34';
      context.lineWidth = 1.5;
      for (const edge of data.edges)
        if (edge.from === selected || edge.to === selected) {
          const from = point(byId.get(edge.from)!),
            to = point(byId.get(edge.to)!);
          context.moveTo(from.x, from.y);
          context.lineTo(to.x, to.y);
        }
      context.stroke();
    }
    for (const node of visible) {
      const p = point(node);
      if (!onScreen(p)) continue;
      const active = node.id === selected,
        neighbor = related.has(node.id);
      const radius = active
        ? 5
        : !node.formalized
          ? 4
          : neighbor
            ? 3
            : Math.min(3, 0.6 + viewport.zoom * 0.12);
      context.beginPath();
      context.arc(p.x, p.y, radius, 0, 2 * Math.PI);
      context.fillStyle = active ? '#9c6b34' : neighbor ? '#365d4e' : '#557965';
      context.strokeStyle = active ? '#9c6b34' : '#557965';
      if (node.formalized) context.fill();
      else {
        context.lineWidth = 1.5;
        context.stroke();
      }
      if (active || neighbor || viewport.zoom > 12) {
        context.fillStyle = '#26352e';
        context.font = '12px sans-serif';
        context.fillText(node.title, p.x + 7, p.y + 4, 260);
      }
    }
  }, [byId, data, edges, geometry, related, selected, viewport, visible, width]);
  function focus(id: string) {
    const node = byId.get(id);
    if (!node) return;
    setDomain('');
    setSelected(id);
    setQuery('');
    setViewport({ zoom: 5, x: 0, y: 0, anchor: id });
  }
  const neighbors = (direction: 'before' | 'after') =>
    data.edges
      .filter((edge) => (direction === 'before' ? edge.to === selected : edge.from === selected))
      .map((edge) => byId.get(direction === 'before' ? edge.from : edge.to)!)
      .filter(Boolean);
  return (
    <section className="shared-library" aria-label={t('Shared mathematical library')}>
      <div className="shared-library-toolbar">
        <p>
          {t('{count} formalized modules', { count: formalized.toLocaleString() })}
          {' · '}
          {t('{count} open', { count: data.nodes.length - formalized })}
        </p>
        <label>
          {t('Subject')}{' '}
          <select
            value={domain}
            onChange={(event) => {
              setDomain(event.target.value);
              setViewport({ zoom: 1, x: 0, y: 0 });
            }}
          >
            <option value="">{t('All fields')}</option>
            {domains.map((domain) => (
              <option key={domain}>{domain}</option>
            ))}
          </select>
        </label>
        <button className="text-button" type="button" onClick={onRefresh}>
          {t('Refresh library')}
        </button>
      </div>
      <p className="shared-library-intro">
        {t(
          'Explore the shared Lean library and its published dependencies. Select a node to read the mathematics and see what builds on it.',
        )}
      </p>
      <div className="shared-library-workspace">
        <div className="shared-library-map">
          <div className="shared-library-controls" aria-label={t('Graph controls')}>
            <button
              className="text-button"
              aria-label={t('Zoom in')}
              onClick={() =>
                setViewport((previous) => ({
                  ...previous,
                  zoom: Math.min(100, previous.zoom * 1.5),
                }))
              }
            >
              +
            </button>
            <button
              className="text-button"
              aria-label={t('Zoom out')}
              onClick={() =>
                setViewport((previous) => ({
                  ...previous,
                  zoom: Math.max(0.5, previous.zoom / 1.5),
                }))
              }
            >
              −
            </button>
            <button className="text-button" onClick={() => setViewport({ zoom: 1, x: 0, y: 0 })}>
              {t('Fit graph')}
            </button>
            <span>
              <i className="shared-library-dot" />
              {t('Formalized')}
              {'  '}
              <i className="shared-library-dot is-open" />
              {t('Open')}
            </span>
          </div>
          <canvas
            ref={canvas}
            role="img"
            tabIndex={0}
            aria-label={t('Shared Lean dependency graph. Drag to move and scroll to zoom.')}
            style={{ width: '100%', height: HEIGHT }}
            onPointerDown={(event) => {
              event.currentTarget.setPointerCapture(event.pointerId);
              drag.current = { x: event.clientX, y: event.clientY, moved: false };
            }}
            onPointerMove={(event) => {
              if (!drag.current) return;
              const dx = event.clientX - drag.current.x,
                dy = event.clientY - drag.current.y;
              if (Math.abs(dx) + Math.abs(dy) > 2) drag.current.moved = true;
              drag.current.x = event.clientX;
              drag.current.y = event.clientY;
              setViewport((previous) => ({ ...previous, x: previous.x + dx, y: previous.y + dy }));
            }}
            onPointerUp={(event) => {
              if (drag.current && !drag.current.moved) {
                const origin = byId.get(viewport.anchor ?? '') ?? geometry;
                const rect = event.currentTarget.getBoundingClientRect(),
                  x = event.clientX - rect.left,
                  y = event.clientY - rect.top;
                let chosen: LibraryNode | undefined,
                  distance = 12;
                for (const node of visible) {
                  const dx =
                      width / 2 +
                      viewport.x +
                      (node.x - origin.x) * geometry.scale * viewport.zoom -
                      x,
                    dy =
                      HEIGHT / 2 +
                      viewport.y +
                      (node.y - origin.y) * geometry.scale * viewport.zoom -
                      y,
                    d = Math.hypot(dx, dy);
                  if (d < distance) {
                    chosen = node;
                    distance = d;
                  }
                }
                if (chosen) setSelected(chosen.id);
              }
              drag.current = null;
            }}
            onPointerCancel={() => {
              drag.current = null;
            }}
            onKeyDown={(event) => {
              const delta: Record<string, [number, number]> = {
                ArrowLeft: [40, 0],
                ArrowRight: [-40, 0],
                ArrowUp: [0, 40],
                ArrowDown: [0, -40],
              };
              const move = delta[event.key];
              if (move) {
                event.preventDefault();
                setViewport((previous) => ({
                  ...previous,
                  x: previous.x + move[0],
                  y: previous.y + move[1],
                }));
              }
            }}
          />
        </div>
        <aside className="shared-library-detail">
          <label>
            {t('Find a result')}
            <input
              type="search"
              maxLength={200}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t('Title or mathematical topic…')}
            />
          </label>
          {matches.length ? (
            <ul className="shared-library-results">
              {matches.map((node) => (
                <li key={node.id}>
                  <button className="text-button" onClick={() => focus(node.id)}>
                    {node.title}
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
          {node ? (
            <>
              <h3>{node.title}</h3>
              <p className="muted">
                {node.domain}
                {' · '}
                {t(node.formalized ? 'Formalized' : 'Open')}
              </p>
              {node.abstract ? <LatexText source={node.abstract} /> : null}
              <p>
                <ExternalLink href={node.source}>{t('Lean source')}</ExternalLink>
              </p>
              {linkedPapers.length ? (
                <section>
                  <h4>{t('Papers with this Lean source')}</h4>
                  <ul>
                    {linkedPapers.map((paper) => (
                      <li key={paper.id}>
                        <Link to={{ kind: 'paper', record: paper.id, version: paper.revision }}>
                          {paper.title}
                        </Link>
                      </li>
                    ))}
                  </ul>
                </section>
              ) : null}
              {(['before', 'after'] as const).map((direction) => {
                const items = neighbors(direction);
                return items.length ? (
                  <section key={direction}>
                    <h4>
                      {t(direction === 'before' ? 'Builds on' : 'Used by')} ({items.length})
                    </h4>
                    <ul>
                      {items.slice(0, 20).map((neighbor) => (
                        <li key={neighbor.id}>
                          <button className="text-button" onClick={() => focus(neighbor.id)}>
                            {neighbor.title}
                          </button>
                        </li>
                      ))}
                    </ul>
                    {items.length > 20 ? (
                      <details>
                        <summary>{t('Show all')}</summary>
                        <ul>
                          {items.slice(20).map((neighbor) => (
                            <li key={neighbor.id}>
                              <button className="text-button" onClick={() => focus(neighbor.id)}>
                                {neighbor.title}
                              </button>
                            </li>
                          ))}
                        </ul>
                      </details>
                    ) : null}
                  </section>
                ) : null;
              })}
            </>
          ) : (
            <p className="muted">
              {t('Select a node to explore its result, prerequisites and later uses.')}
            </p>
          )}
          <p>
            <ExternalLink href="https://www.trureturing.org/atlas.html">
              {t('Open the full library atlas')}
            </ExternalLink>
          </p>
        </aside>
      </div>
    </section>
  );
}
