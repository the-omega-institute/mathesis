import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useAsync } from '../api/useAsync';
import { useApi } from '../api/context';
import { listDirectoryNotes } from '../lib/paperDirectory';
import type { CollectedPaper } from '../api/types';
import {
  formalizationLibrary,
  formalizationPaperSubjects,
  primaryFormalizationSubject,
  layoutFormalizationLibrary,
  type FormalizationLibrary,
  type FormalizationNode,
} from '../lib/formalizationLibrary';
import { findSubject, subjectGroups } from '../lib/subjects';
import { Link, useRouter } from '../routing/router';
import '../styles/subjects.css';
import { paperRoute } from '../routing/routes';
import { t, useLocale } from '../i18n/locale';
import { loadSharedLibrary } from '../lib/sharedLibrary';
import { Async, ExternalLink } from './ui';
import { LatexText } from './Markdown';

export function SharedLibraryGraph({ focus, msc }: { focus?: string; msc?: string }) {
  const api = useApi();
  const papers = useAsync(
    useCallback((signal: AbortSignal) => listDirectoryNotes(api, {}, signal), [api]),
  );
  return (
    <Async state={papers.state} onRetry={papers.reload}>
      {(page) =>
        msc || focus ? (
          <SubjectGraphSource
            papers={page.items}
            msc={msc}
            focus={focus}
            onRefresh={papers.reload}
          />
        ) : (
          <SubjectDirectory papers={page.items} />
        )
      }
    </Async>
  );
}

function SubjectDirectory({ papers }: { papers: CollectedPaper[] }) {
  const counts = new Map<string, number>();
  for (const paper of papers)
    for (const code of formalizationPaperSubjects(paper))
      counts.set(code, (counts.get(code) ?? 0) + 1);
  return (
    <section className="formalization-subjects" aria-label={t('Subject graphs')}>
      <h2>{t('Browse by subject')}</h2>
      <p className="shared-library-intro">
        {t(
          'Each subject has its own dependency graph. Choose a subject to explore its results and the mathematics they build on.',
        )}
      </p>
      <div className="subject-directory">
        {subjectGroups.map((group) => {
          const subjects = group.subjects.filter((subject) => counts.has(subject.code));
          return subjects.length ? (
            <section className="subject-group" key={group.title}>
              <h3>{t(group.title)}</h3>
              <ul>
                {subjects.map((subject) => (
                  <li key={subject.code}>
                    <Link to={{ kind: 'formalizations', msc: subject.code }}>
                      {t(subject.title)}
                    </Link>
                    <span className="muted">
                      {' '}
                      ·{' '}
                      {counts.get(subject.code) === 1
                        ? t('1 paper')
                        : t('{count} papers', { count: counts.get(subject.code)! })}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ) : null;
        })}
      </div>
      {!counts.size ? <p>{t('No formalizations are listed in this subject yet.')}</p> : null}
    </section>
  );
}

function SubjectGraphSource({
  papers,
  focus,
  msc,
  onRefresh,
}: {
  papers: CollectedPaper[];
  focus?: string;
  msc?: string;
  onRefresh: () => void;
}) {
  const api = useApi();
  const library = useAsync(
    useCallback(
      async (signal: AbortSignal) => {
        const selectedPapers = msc
          ? papers.filter((paper) => formalizationPaperSubjects(paper).includes(msc))
          : papers;
        const source = await loadSharedLibrary(signal);
        const knowledge = [];
        for (let start = 0; start < selectedPapers.length; start += 6) {
          const batch = await Promise.allSettled(
            selectedPapers
              .slice(start, start + 6)
              .map((paper) => api.collectedKnowledge(paper.id, paper.revision, { signal })),
          );
          for (const result of batch)
            if (result.status === 'fulfilled') knowledge.push(result.value);
        }
        if (signal.aborted) throw signal.reason;
        return formalizationLibrary(
          source,
          selectedPapers,
          knowledge,
          msc ? undefined : focus,
          papers,
        );
      },
      [api, papers, focus, msc],
    ),
  );
  return (
    <Async state={library.state} onRetry={onRefresh}>
      {(data) => (
        <SubjectGraph data={data} papers={papers} msc={msc} focus={focus} onRefresh={onRefresh} />
      )}
    </Async>
  );
}

function SubjectGraph({
  data,
  papers,
  msc,
  focus,
  onRefresh,
}: {
  data: FormalizationLibrary;
  papers: CollectedPaper[];
  msc?: string;
  focus?: string;
  onRefresh: () => void;
}) {
  const { navigate } = useRouter();
  const target = data.nodes.find((node) => node.id === focus);
  const subject = msc ?? (target ? primaryFormalizationSubject(target) : undefined);
  useEffect(() => {
    if (!msc && subject)
      navigate(
        { kind: 'formalizations', msc: subject, ...(focus ? { focus: 'module:' + focus } : {}) },
        { replace: true },
      );
  }, [msc, subject, focus, navigate]);
  return subject ? (
    <LibraryExplorer
      key={subject}
      data={data}
      domain={subject}
      papers={papers}
      initialFocus={focus}
      onRefresh={onRefresh}
    />
  ) : (
    <SubjectDirectory papers={papers} />
  );
}
const HEIGHT = 600;
function frame(nodes: { x: number; y: number }[], width: number) {
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
  domain,
  papers,
}: {
  data: FormalizationLibrary;
  onRefresh: () => void;
  initialFocus?: string;
  domain: string;
  papers: CollectedPaper[];
}) {
  const locale = useLocale();
  const { navigate } = useRouter();
  const navigateSubject = (msc: string) => navigate({ kind: 'formalizations', msc });
  const [selected, setSelected] = useState(initialFocus ?? '');
  const [viewport, setViewport] = useState<{ zoom: number; x: number; y: number; anchor?: string }>(
    { zoom: initialFocus ? 5 : 1, x: 0, y: 0, anchor: initialFocus },
  );
  const [width, setWidth] = useState(900);
  const canvas = useRef<HTMLCanvasElement>(null);
  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const layout = useMemo(() => layoutFormalizationLibrary(data, domain), [data, domain]);
  const visible = layout.nodes;
  const byId = useMemo(() => new Map(visible.map((node) => [node.id, node])), [visible]);
  const ids = useMemo(() => new Set(visible.map((node) => node.id)), [visible]);
  const edges = useMemo(
    () => data.edges.filter((edge) => ids.has(edge.from) && ids.has(edge.to)),
    [data, ids],
  );
  const geometry = useMemo(() => frame(visible, width), [visible, width]);
  const node = byId.get(selected);
  const linkedPapers = node?.papers ?? [];
  const related = useMemo(
    () =>
      new Set(
        edges
          .filter((edge) => edge.from === selected || edge.to === selected)
          .flatMap((edge) => [edge.from, edge.to]),
      ),
    [edges, selected],
  );
  const paperNodes = visible.filter(
    (node) => node.subjects.includes(domain) && node.papers.length && !node.claim,
  );
  const formalized = visible.filter((node) => node.formalized).length;

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
    const point = (node: { x: number; y: number }) => ({
      x: width / 2 + viewport.x + (node.x - origin.x) * geometry.scale * viewport.zoom,
      y: HEIGHT / 2 + viewport.y + (node.y - origin.y) * geometry.scale * viewport.zoom,
    });
    const onScreen = (point: { x: number; y: number }) =>
      point.x >= -20 && point.x <= width + 20 && point.y >= -20 && point.y <= HEIGHT + 20;
    context.lineWidth = 0.5;
    context.strokeStyle = '#7e948926';
    for (const contextual of [false, true]) {
      context.setLineDash(contextual ? [3, 4] : []);
      context.beginPath();
      for (const edge of edges) {
        if (Boolean(edge.context) !== contextual) continue;
        const from = point(byId.get(edge.from)!),
          to = point(byId.get(edge.to)!);
        if (!onScreen(from) && !onScreen(to)) continue;
        context.moveTo(from.x, from.y);
        context.lineTo(to.x, to.y);
      }
      context.stroke();
    }
    if (selected) {
      context.strokeStyle = '#9c6b34';
      context.lineWidth = 1.5;
      for (const edge of edges)
        if (edge.from === selected || edge.to === selected) {
          const from = point(byId.get(edge.from)!),
            to = point(byId.get(edge.to)!);
          context.setLineDash(edge.context ? [3, 4] : []);
          context.beginPath();
          context.moveTo(from.x, from.y);
          context.lineTo(to.x, to.y);
          context.stroke();
          if (!edge.context) {
            const angle = Math.atan2(to.y - from.y, to.x - from.x);
            context.setLineDash([]);
            context.beginPath();
            context.moveTo(to.x - 5 * Math.cos(angle), to.y - 5 * Math.sin(angle));
            context.lineTo(to.x - 12 * Math.cos(angle - 0.35), to.y - 12 * Math.sin(angle - 0.35));
            context.lineTo(to.x - 12 * Math.cos(angle + 0.35), to.y - 12 * Math.sin(angle + 0.35));
            context.closePath();
            context.fillStyle = '#9c6b34';
            context.fill();
          }
        }
    }
    context.setLineDash([]);
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
            : node.papers.length
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
  }, [
    byId,
    data,
    edges,
    geometry,
    related,
    selected,
    viewport,
    visible,
    width,
    domain,
    layout,
    locale,
  ]);
  function focus(id: string) {
    const node = byId.get(id);
    if (!node) return;
    setSelected(id);
    setViewport({ zoom: 1.4, x: 0, y: 0, anchor: id });
  }
  const neighbors = (direction: 'before' | 'after') =>
    edges
      .filter(
        (edge) =>
          !edge.context && (direction === 'before' ? edge.to === selected : edge.from === selected),
      )
      .map((edge) => byId.get(direction === 'before' ? edge.from : edge.to)!)
      .filter(Boolean);
  return (
    <section className="shared-library" aria-label={t('Mathematical knowledge')}>
      <nav className="subject-navigation" aria-label={t('Subject graphs')}>
        <Link to={{ kind: 'formalizations' }}>{t('All subjects')}</Link>
        <Link to={{ kind: 'subject', msc: domain }}>{t('Research papers')}</Link>
      </nav>
      <div className="shared-library-toolbar">
        <div>
          <h2>{t(findSubject(domain)!.title)}</h2>
          <p>
            {t('{count} formalized modules', { count: formalized.toLocaleString() })}
            {visible.length > formalized ? (
              <> · {t('{count} open', { count: visible.length - formalized })}</>
            ) : null}
          </p>
        </div>
        <label>
          {t('Subject')}{' '}
          <select value={domain} onChange={(event) => navigateSubject(event.target.value)}>
            {subjectGroups.map((group) => {
              const subjects = group.subjects.filter(
                (subject) =>
                  subject.code === domain ||
                  papers.some((paper) => formalizationPaperSubjects(paper).includes(subject.code)),
              );
              return subjects.length ? (
                <optgroup key={group.title} label={t(group.title)}>
                  {subjects.map((subject) => (
                    <option key={subject.code} value={subject.code}>
                      {t(subject.title)}
                    </option>
                  ))}
                </optgroup>
              ) : null;
            })}
          </select>
        </label>
        <button className="text-button" type="button" onClick={onRefresh}>
          {t('Refresh library')}
        </button>
      </div>
      <p className="shared-library-intro">
        {t(
          'This graph shows results in this subject and their prerequisites. Shared foundations appear where they are used.',
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
            aria-label={t('Mathematical dependency graph. Drag to move and scroll to zoom.')}
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
                let chosen: FormalizationNode | undefined,
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
          {!visible.length ? <p>{t('No formalizations are listed in this subject yet.')}</p> : null}
        </div>
        <aside className="shared-library-detail">
          {node ? (
            <>
              <h3>{node.title}</h3>
              <p className="muted">
                {node.subjects.map((code) => t(findSubject(code)!.title)).join(' · ')}
                {' · '}
                {t(node.formalized ? 'Formalized' : 'Open')}
              </p>
              {!node.subjects.includes(domain) ? (
                <p className="muted">{t('Prerequisite from another subject')}</p>
              ) : null}
              {node.abstract ? <LatexText source={node.abstract} /> : null}
              <p>
                {node.claim && linkedPapers[0] ? (
                  <Link
                    to={{
                      kind: 'result',
                      record: linkedPapers[0].id,
                      version: linkedPapers[0].revision,
                      claim: node.claim,
                    }}
                  >
                    {t('Read result')}
                  </Link>
                ) : (
                  <ExternalLink href={node.source}>{t('Lean source')}</ExternalLink>
                )}
              </p>
              {linkedPapers.length ? (
                <section>
                  <h4>{t('Related papers')}</h4>
                  <ul>
                    {linkedPapers.map((paper) => (
                      <li key={paper.id}>
                        <Link
                          to={paperRoute(
                            paper.id,
                            paper.archive?.doi,
                            paper.archive?.doi ? undefined : paper.revision,
                          )}
                        >
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
          {linkedPapers.length ? <PaperRequests papers={linkedPapers} /> : null}
          {!node && paperNodes.length ? (
            <section>
              <h3>{t('Papers in this graph')}</h3>
              <ul className="shared-library-results">
                {paperNodes.slice(0, 8).map((item) => (
                  <li key={item.id}>
                    <button className="text-button" onClick={() => focus(item.id)}>
                      {item.papers[0]!.title}
                    </button>
                  </li>
                ))}
              </ul>
              {paperNodes.length > 8 ? (
                <details>
                  <summary>
                    {t('Show all')} ({paperNodes.length})
                  </summary>
                  <ul className="shared-library-results">
                    {paperNodes.slice(8).map((item) => (
                      <li key={item.id}>
                        <button className="text-button" onClick={() => focus(item.id)}>
                          {item.papers[0]!.title}
                        </button>
                      </li>
                    ))}
                  </ul>
                </details>
              ) : null}
            </section>
          ) : null}
          {domain ? (
            <p>
              <Link to={{ kind: 'subject', msc: domain }}>{t('Research papers')}</Link>
            </p>
          ) : null}
        </aside>
      </div>
    </section>
  );
}

function PaperRequests({ papers }: { papers: CollectedPaper[] }) {
  const requests = [
    ...new Map(
      papers
        .flatMap((paper) => paper.materials?.formalization_requests ?? [])
        .map((request) => [request.url, request]),
    ).values(),
  ];
  if (!requests.length) return null;
  return (
    <section>
      <h4>{t('Formalization requests')}</h4>
      <ul>
        {requests.map((request) => (
          <li key={request.url}>
            <ExternalLink href={request.url}>{request.label}</ExternalLink>
          </li>
        ))}
      </ul>
    </section>
  );
}
