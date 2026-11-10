import '../styles/formalizations.css';
import { t } from '../i18n/locale';
import { InlineLatex } from '../components/Markdown';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { usePaged } from '../api/usePaged';
import { Async, Badge } from '../components/ui';
import { LatexText, MacrosProvider } from '../components/Markdown';
import { ProofAttribution } from '../components/ProofAttribution';
import { Pager } from '../components/Pager';
import { Link } from '../routing/router';
import { AuthorLine } from '../components/AuthorLine';
import { EntrantName } from './Leaderboard';
import { ResearchGraphPage } from './ResearchGraph';
import { useRouter } from '../routing/router';
import { useEffect } from 'react';
import { ApiError } from '../api/client';
import { publishedLeanSource } from '../lib/publishedLean';

export function FormalizationsPage({
  focus,
  q,
  before,
  view,
}: {
  focus?: string;
  q?: string;
  before?: string;
  view?: 'list';
}) {
  const { route, navigate } = useRouter();
  useEffect(() => {
    if (route.kind === 'research_graph')
      navigate({ kind: 'formalizations', focus, q, before }, { replace: true });
  }, [route.kind, navigate, focus, q, before]);
  return (
    <div className="page formalizations-page">
      <header className="page-head formalizations-head">
        <div>
          <h1>{t('Formalizations')}</h1>
          <p className="lede">
            {t(
              'Explore how results connect, find a missing formal proof, and contribute Lean work with credit for the mathematics and its formalization.',
            )}
          </p>
        </div>
        <nav className="formalization-actions" aria-label={t('Formalization contributions')}>
          <Link className="button" to={{ kind: 'submit', intake: 'lean' }}>
            {t('Contribute Lean work')}
          </Link>
          <Link to={{ kind: 'leaderboard', category: 'formalization' }}>{t('Leaderboard')}</Link>
        </nav>
      </header>
      <nav className="section-tabs" aria-label={t('Formalization views')}>
        <Link to={{ kind: 'formalizations' }} aria-current={view !== 'list' ? 'page' : undefined}>
          {t('Dependency graph')}
        </Link>
        <Link
          to={{ kind: 'formalizations', view: 'list' }}
          aria-current={view === 'list' ? 'page' : undefined}
        >
          {t('Results and proof work')}
        </Link>
      </nav>
      {view === 'list' ? (
        <FormalizationOpportunities initialQuery={q} />
      ) : (
        <ResearchGraphPage embedded focus={focus} q={q} before={before} />
      )}
    </div>
  );
}

export function FormalizationOpportunities({ initialQuery = '' }: { initialQuery?: string }) {
  const api = useApi();
  const [status, setStatus] = useState('unverified');
  const [kind, setKind] = useState('');
  const [draft, setDraft] = useState(initialQuery);
  const [query, setQuery] = useState(initialQuery);
  const load = useCallback(
    async (before: string | null, signal: AbortSignal) => {
      const page = await api.formalizationOpportunities(
        { before, limit: 20, status, kind: kind || undefined, q: query || undefined },
        { signal },
      );
      const sources = new Map<string, Promise<string | undefined>>();
      const items = await Promise.all(
        page.items.map(async (item) => {
          let leanSource: string | undefined;
          if (!item.record.startsWith('WP-') && item.version) {
            const key = `${item.record}:${item.version}`;
            if (!sources.has(key))
              sources.set(
                key,
                api
                  .getCollectedPaper(item.record, String(item.version), { signal })
                  .then((paper) =>
                    paper.id === item.record && paper.revision === String(item.version)
                      ? publishedLeanSource(paper)
                      : undefined,
                  )
                  .catch((error: unknown) => {
                    if (error instanceof ApiError && error.code === 'not_found') return undefined;
                    throw error;
                  }),
              );
            leanSource = await sources.get(key);
          }
          return { ...item, leanSource };
        }),
      );
      return { ...page, items };
    },
    [api, status, kind, query],
  );
  const paged = usePaged(load);
  const search = (event: FormEvent) => {
    event.preventDefault();
    setQuery(draft.trim());
  };
  return (
    <section className="formalization-work" aria-label={t('Formalization work')}>
      <p>
        {t(
          'Read the mathematics and its existing Lean sources. Contribute additional proofs where an exact target is available, or share existing Lean work with its original attribution.',
        )}
      </p>
      <div className="filter-row" role="group" aria-label={t('Formalization status')}>
        {[
          ['unverified', 'Explore results'],
          ['verified', 'Verified proofs'],
          ['disproved', 'Disproofs'],
        ].map(([value, label]) => (
          <button
            key={value}
            className="filter-chip"
            aria-pressed={status === value}
            onClick={() => setStatus(value!)}
          >
            {t(label ?? '')}
          </button>
        ))}
      </div>
      <form className="formalization-search" role="search" onSubmit={search}>
        <label htmlFor="formalization-query" className="visually-hidden">
          {t('Find a statement')}
        </label>
        <input
          id="formalization-query"
          type="search"
          maxLength={200}
          placeholder={t('Search statements, papers, or authors')}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <label htmlFor="formalization-kind" className="visually-hidden">
          {t('Statement type')}
        </label>
        <select id="formalization-kind" value={kind} onChange={(e) => setKind(e.target.value)}>
          <option value="">{t('All statements')}</option>
          <option value="paper">{t('Paper results')}</option>
          <option value="conjecture">{t('Conjectures')}</option>
        </select>
        <button className="button button-quiet">{t('Search')}</button>
      </form>
      <Async state={paged.state} onRetry={paged.reload}>
        {(page) =>
          page.items.length ? (
            <ul className="entry-list">
              {page.items.map((item) => (
                <li key={`${item.record}:${item.claim}:${item.version ?? ''}`}>
                  <MacrosProvider macros={item.macros ?? {}}>
                    <article>
                      <p className="entry-meta">
                        <Badge
                          tone={['preparing', 'open'].includes(item.status) ? 'muted' : 'good'}
                        >
                          {item.status === 'preparing'
                            ? item.leanSource
                              ? t('Lean source available')
                              : t('Awaiting formalization')
                            : item.status === 'open'
                              ? t('Target ready')
                              : item.status === 'verified'
                                ? t('Lean ✓')
                                : t('Lean disproof')}
                        </Badge>
                      </p>
                      <Link
                        className="entry-title"
                        to={{
                          kind: item.status === 'preparing' ? 'result' : 'statement',
                          record: item.record,
                          claim: item.claim,
                          ...(item.status === 'preparing'
                            ? { version: item.version?.toString() }
                            : {}),
                        }}
                      >
                        <InlineLatex source={item.label} />
                      </Link>
                      <p>
                        <Link
                          to={{
                            kind: 'paper',
                            record: item.record,
                            version: item.version?.toString(),
                          }}
                        >
                          <InlineLatex source={item.title} />
                        </Link>
                      </p>
                      <AuthorLine authors={item.authors} compact />
                      <LatexText source={item.statement} className="result-excerpt" />
                      {item.contributor ? (
                        <p className="small">
                          {t('Verified contribution by ')}
                          <EntrantName entrant={item.contributor} /> · {item.verified_count}{' '}
                          {t(' verified')} {item.verified_count === 1 ? 'attempt' : 'attempts'}
                        </p>
                      ) : null}
                      <ProofAttribution attribution={item.attribution ?? undefined} />
                      <div className="button-row">
                        {item.leanSource ? (
                          <a href={item.leanSource}>{t('Read the Lean source')}</a>
                        ) : null}
                        {item.target_digest ? (
                          <>
                            <Link
                              className="button button-small"
                              to={{
                                kind: 'submit',
                                intake: 'lean',
                                record: item.record,
                                claim: item.claim,
                              }}
                            >
                              {item.status === 'open'
                                ? t('Contribute a proof')
                                : t('Add another proof')}
                            </Link>
                            <Link
                              className="text-link"
                              to={{ kind: 'statement', record: item.record, claim: item.claim }}
                            >
                              {t('View statement & ')}
                              {item.status === 'open' ? 'target' : 'receipt'} →
                            </Link>
                          </>
                        ) : (
                          <Link
                            className="text-link"
                            to={{
                              kind: 'result',
                              record: item.record,
                              claim: item.claim,
                              version: item.version?.toString(),
                            }}
                          >
                            {t('Read result')} →
                          </Link>
                        )}
                      </div>
                    </article>
                  </MacrosProvider>
                </li>
              ))}
            </ul>
          ) : (
            <div className="empty-state">
              <h2>
                {query || kind
                  ? t('No matching statements')
                  : status === 'unverified'
                    ? t('No mathematical results listed yet')
                    : t('Verified contributions will appear here.')}
              </h2>
              <p>
                {status === 'unverified'
                  ? t(
                      'Published mathematical statements appear here with their existing Lean sources and any confirmed proof targets.',
                    )
                  : t(
                      'Only independent verification of the current confirmed target makes a contribution public here.',
                    )}
              </p>
              <Link className="text-link" to={{ kind: 'submit', intake: 'lean' }}>
                {t('Submit existing Lean work →')}
              </Link>
            </div>
          )
        }
      </Async>
      <Pager paged={paged} />
    </section>
  );
}
