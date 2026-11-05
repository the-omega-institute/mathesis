import { useCallback, useId, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { CollectedKnowledge, CollectedPaper, PublicPaper } from '../api/types';
import { InlineLatex, LatexText, MacrosProvider } from './Markdown';
import { Async } from './ui';
import { Link } from '../routing/router';
import { t } from '../i18n/locale';
import { safeHttpUrl } from '../lib/links';

export type KnowledgeResult = CollectedKnowledge['statements'][number] & {
  verified?: boolean;
  target?: boolean;
};
export type KnowledgeReading = {
  work: string;
  version: string;
  title: string;
  authors: { name: string }[];
  collected: boolean;
  statements: KnowledgeResult[];
  connections: { from: string; to: string }[];
  macros: Record<string, string>;
  lean?: string;
  limitations: string[];
};
type Work = { id: string; title: string; collected: boolean; paper?: CollectedPaper };

function ordinaryReading(paper: PublicPaper): KnowledgeReading {
  const claims = paper.claims ?? [];
  return {
    work: paper.summary.record,
    version: String(paper.publication?.version ?? 1),
    title: paper.summary.title,
    authors: paper.summary.authors,
    collected: false,
    statements: claims.map((claim) => ({
      id: claim.id,
      kind: claim.kind,
      name: claim.label.replace(/\s*\[[^\]]*\]\s*$/, ''),
      statement: claim.statement,
      section: claim.section,
      verified: Boolean(claim.verified),
      target: Boolean(paper.lean_statements?.some((target) => target.claim === claim.id)),
    })),
    connections: claims.flatMap((claim) =>
      claim.depends_on
        .filter((id) => claims.some((c) => c.id === id))
        .map((id) => ({ from: claim.id, to: id })),
    ),
    macros: paper.macros ?? {},
    limitations: [],
  };
}

export function MathematicalKnowledge({ focus }: { focus?: string }) {
  const api = useApi();
  const [cursor, setCursor] = useState<{ lane: 'collected' | 'papers'; before?: string }>({
    lane: 'collected',
  });
  const [chosen, setChosen] = useState<string>();
  const [followFocus, setFollowFocus] = useState(true);
  const loadWorks = useCallback(
    async (signal: AbortSignal) => {
      const ordinary = async () => {
        const page = await api.listPapers(
          { limit: 12, before: cursor.lane === 'papers' ? cursor.before : undefined },
          { signal },
        );
        return {
          works: page.items
            .filter((paper) => Boolean(paper.accepted_at))
            .map((paper): Work => ({
              id: paper.record,
              title: paper.title,
              collected: false,
            })),
          next: page.next_before
            ? { lane: 'papers' as const, before: page.next_before }
            : undefined,
        };
      };
      if (cursor.lane === 'papers') return ordinary();
      const page = await api.listCollectedPapers({ limit: 12, before: cursor.before }, { signal });
      if (!page.items.length && !page.next_before) return ordinary();
      const ordinaryPage = page.next_before ? undefined : await ordinary();
      return {
        works: page.items.map((paper): Work => ({
          id: paper.id,
          title: paper.title,
          collected: true,
          paper,
        })),
        next: page.next_before
          ? { lane: 'collected' as const, before: page.next_before }
          : ordinaryPage && (ordinaryPage.works.length || ordinaryPage.next)
            ? { lane: 'papers' as const }
            : undefined,
      };
    },
    [api, cursor],
  );
  const works = useAsync(loadWorks);
  const exact = /^statement:([^:]+):([1-9]\d*):([^:]+)$/.exec(focus ?? '');
  const exactVersion = followFocus && !chosen ? exact?.[2] : undefined;
  const selectedWork =
    works.value?.works.find((work) => work.id === chosen) ?? works.value?.works[0];
  const work = chosen ?? (followFocus ? exact?.[1] : undefined) ?? selectedWork?.id;
  const source = works.value?.works.find((w) => w.id === work);
  const loadReading = useCallback(
    async (signal: AbortSignal): Promise<KnowledgeReading> => {
      if (!work) throw new Error('Choose a paper.');
      if (source?.collected === false || work.startsWith('WP-'))
        return ordinaryReading(await api.getPaper(work, { signal }));
      const paper =
        source?.paper && !exactVersion
          ? source.paper
          : await api.getCollectedPaper(work, exactVersion, { signal });
      const knowledge = await api.collectedKnowledge(work, paper.revision, { signal });
      return {
        work,
        version: paper.revision,
        title: paper.title,
        authors: paper.authors,
        collected: true,
        statements: knowledge.statements,
        connections: knowledge.connections,
        macros: knowledge.macros,
        lean: paper.lean?.url,
        limitations: knowledge.limitations,
      };
    },
    [api, work, source, exactVersion],
  );
  const reading = useAsync(work ? loadReading : null);
  const selectId = useId();
  return (
    <Async state={works.state} onRetry={works.reload}>
      {(data) =>
        data.works.length || work || data.next ? (
          <>
            <div className="knowledge-work-picker">
              <label htmlFor={selectId}>{t('Explore a paper')}</label>
              <select
                id={selectId}
                value={work}
                onChange={(event) => {
                  setChosen(event.target.value);
                  setFollowFocus(false);
                }}
              >
                {!data.works.some((item) => item.id === work) && work ? (
                  <option value={work}>{reading.value?.title ?? work}</option>
                ) : null}
                {data.works.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.title}
                  </option>
                ))}
              </select>
              {data.next ? (
                <button
                  type="button"
                  className="text-button"
                  onClick={() => {
                    setCursor(data.next!);
                    setChosen(undefined);
                    setFollowFocus(false);
                  }}
                >
                  {t('More papers')}
                </button>
              ) : null}
              {cursor.before || cursor.lane === 'papers' ? (
                <button
                  type="button"
                  className="text-button"
                  onClick={() => {
                    setCursor({ lane: 'collected' });
                    setChosen(undefined);
                    setFollowFocus(false);
                  }}
                >
                  {t('First papers')}
                </button>
              ) : null}
            </div>
            <Async state={reading.state} onRetry={reading.reload}>
              {(value) => (
                <ResultConstellation
                  key={`${value.work}:${value.version}`}
                  reading={value}
                  initial={followFocus ? exact?.[3] : undefined}
                />
              )}
            </Async>
          </>
        ) : (
          <div className="knowledge-empty">
            <h2>{t('Mathematics to build on')}</h2>
            <p>
              {t(
                'Published results and their connections will appear here. Share a paper or contribute a formal proof to help the library grow.',
              )}
            </p>
            <Link to={{ kind: 'submit' }}>{t('Submit a paper')}</Link>
          </div>
        )
      }
    </Async>
  );
}

/** Local manuscript references are displayed without inventing a proof dependency or a DAG. */
export function ResultConstellation({
  reading,
  initial,
}: {
  reading: KnowledgeReading;
  initial?: string;
}) {
  const [selectedId, setSelectedId] = useState(initial);
  const [page, setPage] = useState(0);
  const arrow = useId();
  const picker = useId();
  const detail = useId();
  const selected =
    reading.statements.find((statement) => statement.id === selectedId) ??
    reading.statements
      .filter((statement) => statement.kind === 'theorem')
      .sort(
        (a, b) =>
          reading.connections.filter((edge) => edge.from === b.id || edge.to === b.id).length -
          reading.connections.filter((edge) => edge.from === a.id || edge.to === a.id).length,
      )[0] ??
    reading.statements[0];
  const byId = new Map(reading.statements.map((statement) => [statement.id, statement]));
  const connections = reading.connections.filter(
    (edge) => byId.has(edge.from) && byId.has(edge.to),
  );
  const nearby = connections.filter(
    (edge) => edge.from === selected?.id || edge.to === selected?.id,
  );
  const references = nearby
    .filter((edge) => edge.from === selected?.id)
    .map((edge) => byId.get(edge.to)!);
  const referenceIds = new Set(references.map((result) => result.id));
  const referencedBy = nearby
    .filter((edge) => edge.to === selected?.id)
    .map((edge) => byId.get(edge.from)!)
    .filter((statement) => !referenceIds.has(statement.id));
  const pages = Math.max(1, Math.ceil(Math.max(references.length, referencedBy.length) / 3));
  const currentPage = Math.min(page, pages - 1);
  const left = references.slice(currentPage * 3, (currentPage + 1) * 3);
  const right = referencedBy.slice(currentPage * 3, (currentPage + 1) * 3);
  const visible = [...left, ...right];
  const positions = new Map<string, { x: number; y: number }>();
  if (selected) positions.set(selected.id, { x: 450, y: 230 });
  for (const [nodes, x] of [
    [left, 145],
    [right, 755],
  ] as const) {
    nodes.forEach((statement, index) =>
      positions.set(statement.id, {
        x,
        y: nodes.length === 1 ? 230 : nodes.length === 2 ? 140 + index * 180 : 80 + index * 150,
      }),
    );
  }
  const choose = (id: string) => {
    setSelectedId(id);
    setPage(0);
  };
  const name = (statement: KnowledgeResult) =>
    statement.title ? `${statement.name}: ${statement.title}` : statement.name;
  const context = (statement: KnowledgeResult) =>
    statement.title ?? statement.section ?? statement.name;
  return (
    <MacrosProvider macros={reading.macros}>
      <header className="knowledge-source-heading">
        <h2>
          <InlineLatex source={reading.title} />
        </h2>
        <p>
          {reading.authors.map((author) => author.name).join('; ')}{' '}
          <span>· {t('Version {version}', { version: reading.version })}</span>
        </p>
      </header>
      {selected ? (
        <>
          <div className="knowledge-result-picker">
            <label htmlFor={picker}>{t('Read a result')}</label>
            <select
              id={picker}
              value={selected.id}
              onChange={(event) => choose(event.target.value)}
            >
              {reading.statements.map((statement, index) => (
                <option key={statement.id} value={statement.id}>
                  {index + 1} · {statement.name}
                  {statement.title || statement.section ? ` — ${context(statement)}` : ''}
                </option>
              ))}
            </select>
          </div>
          <div className="knowledge-result-layout">
            <figure
              className="knowledge-constellation"
              aria-label={t('Mathematical results and connections')}
            >
              <div className="knowledge-map-summary">
                <span>
                  {t('{count} mathematical statements', { count: reading.statements.length })}
                </span>
                <span>
                  {reading.collected
                    ? t('Connections follow references in the manuscript')
                    : t('Connections follow author-declared dependencies')}
                </span>
              </div>
              <div
                className="knowledge-constellation-scroll"
                role="region"
                tabIndex={0}
                aria-label={t('Scrollable result map')}
              >
                <div className="knowledge-constellation-canvas">
                  <svg viewBox="0 0 900 460" preserveAspectRatio="none" aria-hidden="true">
                    <defs>
                      <marker
                        id={arrow}
                        viewBox="0 0 10 10"
                        refX="9"
                        refY="5"
                        markerWidth="6"
                        markerHeight="6"
                        orient="auto"
                      >
                        <path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
                      </marker>
                    </defs>
                    {connections
                      .filter((edge) => positions.has(edge.from) && positions.has(edge.to))
                      .map((edge) => {
                        const a = positions.get(edge.to)!;
                        const b = positions.get(edge.from)!;
                        const direction = Math.sign(b.x - a.x);
                        const startX = a.x + direction * 110;
                        const endX = b.x - direction * 110;
                        const bendY = (a.y + b.y) / 2 + (a.y === b.y ? -38 : 0);
                        const connected = edge.from === selected.id || edge.to === selected.id;
                        return (
                          <path
                            key={`${edge.from}:${edge.to}`}
                            className={`knowledge-result-edge${connected ? ' is-active' : ''}`}
                            data-relation={
                              reading.collected ? 'manuscript_reference' : 'declared_dependency'
                            }
                            d={`M ${startX} ${a.y} Q ${(startX + endX) / 2} ${bendY} ${endX} ${b.y}`}
                            markerEnd={`url(#${arrow})`}
                          />
                        );
                      })}
                  </svg>
                  {[selected, ...visible].map((statement) => {
                    const position = positions.get(statement.id)!;
                    const active = statement.id === selected.id;
                    return (
                      <button
                        type="button"
                        key={statement.id}
                        className={`knowledge-result-node${active ? ' is-selected' : ''}${statement.verified ? ' is-verified' : ''}`}
                        style={{ left: `${position.x / 9}%`, top: position.y }}
                        aria-pressed={active}
                        aria-label={name(statement)}
                        aria-controls={detail}
                        onClick={() => choose(statement.id)}
                      >
                        <span className="knowledge-node-relation">
                          {active
                            ? t('Selected result')
                            : left.some((item) => item.id === statement.id)
                              ? t(reading.collected ? 'References' : 'Declared prerequisite')
                              : t(reading.collected ? 'Referenced by' : 'Used by')}
                        </span>
                        <span className="knowledge-result-kind">{t(statement.name)}</span>
                        <span className="knowledge-result-name">
                          <InlineLatex source={context(statement)} />
                        </span>
                        {statement.verified ? (
                          <span className="knowledge-result-state">{t('Lean verified')}</span>
                        ) : statement.target ? (
                          <span className="knowledge-result-state">
                            {t('Open for a formal proof')}
                          </span>
                        ) : null}
                      </button>
                    );
                  })}
                </div>
              </div>
              <figcaption>
                {t(
                  nearby.length
                    ? 'Select a connected result to follow the argument.'
                    : 'No explicit connections are recorded for this result.',
                )}
              </figcaption>
              {pages > 1 ? (
                <div className="knowledge-result-pagination">
                  <button
                    type="button"
                    className="text-button"
                    disabled={page === 0}
                    onClick={() => setPage(page - 1)}
                  >
                    {t('Previous results')}
                  </button>
                  <span>
                    {t('{page} of {pages}', { page: Math.min(page, pages - 1) + 1, pages })}
                  </span>
                  <button
                    type="button"
                    className="text-button"
                    disabled={page >= pages - 1}
                    onClick={() => setPage(page + 1)}
                  >
                    {t('More results')}
                  </button>
                </div>
              ) : null}
            </figure>
            <section
              id={detail}
              className="knowledge-statement-detail"
              role="region"
              aria-label={t('Selected mathematical statement')}
            >
              <p className="knowledge-detail-kicker">
                {t(
                  selected.kind === 'conjecture' || selected.kind === 'question'
                    ? 'Open question in the manuscript'
                    : 'Result in the manuscript',
                )}
              </p>
              <h3>
                <InlineLatex source={name(selected)} />
              </h3>
              {selected.section ? (
                <p className="knowledge-statement-context">
                  <InlineLatex source={selected.section} />
                </p>
              ) : null}
              <LatexText source={selected.statement} />
              <p className="knowledge-statement-authors">
                {reading.authors.map((author) => author.name).join('; ')}
              </p>
              {nearby.length ? (
                <div className="knowledge-local-links">
                  <h4>{t('Connected results')}</h4>
                  {nearby.map((edge) => {
                    const id = edge.from === selected.id ? edge.to : edge.from;
                    const result = byId.get(id)!;
                    return (
                      <button
                        key={`${edge.from}:${edge.to}`}
                        type="button"
                        className="text-button"
                        onClick={() => choose(id)}
                      >
                        <span>
                          {edge.from === selected.id ? t('References') : t('Referenced by')}
                        </span>{' '}
                        <InlineLatex source={name(result)} />
                        {!result.title && result.section ? (
                          <span className="knowledge-connection-context">
                            <InlineLatex source={result.section} />
                          </span>
                        ) : null}
                      </button>
                    );
                  })}
                </div>
              ) : null}
              <div className="knowledge-proof-state">
                <h4>{t('Formalization')}</h4>
                {selected.verified ? (
                  <p>{t('An independently checked Lean proof is available for this result.')}</p>
                ) : selected.target ? (
                  <>
                    <p>{t('The exact Lean statement is confirmed. You can contribute a proof.')}</p>
                    <Link
                      to={{
                        kind: 'submit',
                        intake: 'lean',
                        record: reading.work,
                        claim: selected.id,
                      }}
                    >
                      {t('Contribute a proof')}
                    </Link>
                  </>
                ) : (
                  <p>{t('A confirmed Lean target is not yet available for this statement.')}</p>
                )}
                {reading.lean && safeHttpUrl(reading.lean) ? (
                  <p>
                    <a href={reading.lean}>{t('Lean source accompanying the paper')}</a>
                    <span className="knowledge-scope-note">
                      {t('Its coverage has not yet been mapped to each statement here.')}
                    </span>
                  </p>
                ) : null}
                <p>
                  <Link to={{ kind: 'leaderboard' }}>{t('See community contributions')}</Link>
                </p>
              </div>
            </section>
          </div>
          <div className="knowledge-reading-links">
            <Link to={{ kind: 'paper', record: reading.work, version: reading.version }}>
              {t('Read the paper and plain-language explanation')}
            </Link>
            {reading.collected ? (
              <Link
                to={{
                  kind: 'research_graph',
                  focus: `statement:${reading.work}:${reading.version}:${selected.id}`,
                }}
              >
                {t('Link to this result')}
              </Link>
            ) : (
              <Link to={{ kind: 'result', record: reading.work, claim: selected.id }}>
                {t('Read the result and contribution history')}
              </Link>
            )}
            <Link to={{ kind: 'research_graph', focus: `paper:${reading.work}` }}>
              {t('Explore the cited literature')}
            </Link>
          </div>
        </>
      ) : (
        <p>
          {t(
            'The statements of this paper are being connected. You can read the full paper in the meantime.',
          )}{' '}
          <Link to={{ kind: 'paper', record: reading.work, version: reading.version }}>
            {t('Read paper')}
          </Link>
        </p>
      )}
      <details className="knowledge-reading-notes">
        <summary>{t('About these connections')}</summary>
        <p>
          {t(
            'Manuscript references show how the written argument is organized. Independently checked Lean proof use is shown separately under Verified proof connections.',
          )}
        </p>
        {reading.limitations.map((limitation, index) => (
          <p key={index}>{t(limitation)}</p>
        ))}
      </details>
    </MacrosProvider>
  );
}
