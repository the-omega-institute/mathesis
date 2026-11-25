import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async, DateText, ExternalLink } from './ui';
import { InlineLatex, Markdown } from './Markdown';
import { t } from '../i18n/locale';
import { Link, useRouter } from '../routing/router';

export function SharedRequests({ initialQuery = '' }: { initialQuery?: string }) {
  const api = useApi();
  const { route } = useRouter();
  const selected = route.kind === 'formalizations' ? route.request : undefined;
  const data = useAsync(
    useCallback((signal: AbortSignal) => api.sharedFormalizationRequests({ signal }), [api]),
  );
  const [query, setQuery] = useState(initialQuery);
  const [status, setStatus] = useState('open');
  const [limit, setLimit] = useState(30);
  return (
    <section aria-label={t('Shared formalization requests')}>
      <p>
        {t('Read the mathematical target, follow the discussion, and see who is contributing.')}
      </p>
      {selected ? (
        <p>
          <Link to={{ kind: 'formalizations', view: 'list' }}>
            {t('Back to awaiting formalization')}
          </Link>
        </p>
      ) : (
        <div className="filter-row">
          <label>
            {t('Search requests')}{' '}
            <input
              type="search"
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setLimit(30);
              }}
            />
          </label>
          <label>
            {t('Status')}{' '}
            <select
              value={status}
              onChange={(event) => {
                setStatus(event.target.value);
                setLimit(30);
              }}
            >
              <option value="open">{t('Open requests')}</option>
              <option value="claimed">{t('Claimed')}</option>
              <option value="merged">{t('Merged contributions')}</option>
              <option value="all">{t('All')}</option>
            </select>
          </label>
        </div>
      )}
      <Async state={data.state} onRetry={data.reload}>
        {(snapshot) => {
          const rows = snapshot.requests.filter((row) =>
            selected
              ? row.number === selected
              : (status === 'all' ||
                  (status === 'open'
                    ? ['open', 'claimed'].includes(row.state)
                    : row.state === status)) &&
                `${row.title}\n${row.body ?? ''}`
                  .toLowerCase()
                  .includes(query.trim().toLowerCase()),
          );
          return (
            <>
              {snapshot.observed_at ? (
                <p className="small muted">
                  {t('Updated')} <DateText iso={snapshot.observed_at} />
                </p>
              ) : (
                <p>
                  {t('Formalization requests are being synchronized. Please check again shortly.')}
                </p>
              )}
              <div className="formalization-request-list">
                {rows.slice(0, limit).map((row) => (
                  <article className="formalization-request-card" key={row.number}>
                    {selected === row.number ? (
                      <h2 className="formalization-request-heading">
                        <span className="formalization-request-number">#{row.number}</span>
                        <InlineLatex source={row.title} />
                      </h2>
                    ) : (
                      <Link
                        to={{ kind: 'formalizations', view: 'list', request: row.number }}
                        className="formalization-request-heading"
                      >
                        <span className="formalization-request-number">#{row.number}</span>
                        <InlineLatex source={row.title} />
                      </Link>
                    )}
                    <p className="entry-meta">
                      {t(
                        row.state === 'claimed'
                          ? 'Claimed'
                          : row.state === 'merged'
                            ? 'Merged contribution'
                            : row.state === 'closed'
                              ? 'Closed request'
                              : 'Open request',
                      )}
                      {row.claimants.length ? (
                        <>
                          {' '}
                          ·{' '}
                          {row.claimants.map((person, index) => (
                            <span key={person.login}>
                              {index ? ', ' : ''}
                              <ExternalLink href={person.url}>{person.login}</ExternalLink>
                            </span>
                          ))}
                        </>
                      ) : null}
                    </p>
                    {selected === row.number ? (
                      <div className="formalization-request-detail">
                        {row.body ? (
                          <Markdown text={row.body} />
                        ) : (
                          <p className="muted">
                            {t('The request description is being synchronized.')}
                          </p>
                        )}
                        <dl className="formalization-request-meta">
                          <div>
                            <dt>{t('Proposed by')}</dt>
                            <dd>{row.requested_by}</dd>
                          </div>
                          {row.paper_repository ? (
                            <div>
                              <dt>{t('Source paper')}</dt>
                              <dd>
                                <ExternalLink href={row.paper_repository}>
                                  {row.paper_repository.split('/').pop()}
                                </ExternalLink>
                              </dd>
                            </div>
                          ) : null}
                          {row.lean_source ? (
                            <div>
                              <dt>{t('Existing Lean source')}</dt>
                              <dd>
                                <ExternalLink href={row.lean_source}>
                                  {t('Open source')}
                                </ExternalLink>
                              </dd>
                            </div>
                          ) : null}
                        </dl>
                        {row.completions.length ? (
                          <div>
                            <h3>{t('Merged proof contributions')}</h3>
                            {row.completions.map((done) => (
                              <p key={done.url}>
                                <ExternalLink href={done.url}>
                                  {t('Read the merged contribution')}
                                </ExternalLink>
                                {' · '}
                                <ExternalLink href={done.contributor.url}>
                                  {done.contributor.login}
                                </ExternalLink>
                                {' · '}
                                <code>{done.revision.slice(0, 8)}</code>
                              </p>
                            ))}
                          </div>
                        ) : null}
                        {row.comments?.length ? (
                          <section
                            className="formalization-request-discussion"
                            aria-label={t('Discussion')}
                          >
                            <h3>{t('Discussion')}</h3>
                            {row.comments.map((comment) => (
                              <article className="formalization-request-comment" key={comment.id}>
                                <p className="entry-meta">
                                  <ExternalLink href={comment.author.url}>
                                    {comment.author.login}
                                  </ExternalLink>
                                  {' · '}
                                  <DateText iso={comment.created_at} withTime />
                                </p>
                                <Markdown text={comment.body} />
                              </article>
                            ))}
                          </section>
                        ) : null}
                        <p className="small formalization-request-source">
                          <ExternalLink href={row.url}>{t('Original on GitHub')}</ExternalLink>
                        </p>
                      </div>
                    ) : null}
                  </article>
                ))}
              </div>
              {rows.length > limit ? (
                <button className="text-button" onClick={() => setLimit(limit + 30)}>
                  {t('Show more')}
                </button>
              ) : null}
              {snapshot.observed_at && !rows.length ? (
                <p>
                  {t(
                    selected
                      ? 'This request is not in the latest synchronized list.'
                      : 'No requests match these filters.',
                  )}
                </p>
              ) : null}
            </>
          );
        }}
      </Async>
    </section>
  );
}

export function SharedRequestLeaderboard({
  period,
  entrants,
}: {
  period: 'all' | 'month';
  entrants: 'all' | 'people' | 'agents';
}) {
  const api = useApi();
  const board = useAsync(
    useCallback(
      (signal: AbortSignal) => api.sharedRequestLeaderboard(period, { signal }),
      [api, period],
    ),
  );
  return (
    <Async state={board.state} onRetry={board.reload}>
      {(data) => {
        const rows = data.rows.filter(
          (row) => entrants === 'all' || row.contributor.agent === (entrants === 'agents'),
        );
        return (
          <section aria-label={t('Shared request contributions')}>
            <p>
              {t(
                'Contributions come from the shared requests and their linked merged pull requests. Claims are participation records; they do not earn points.',
              )}
            </p>
            {!data.observed_at ? (
              <p>
                {t('Formalization requests are being synchronized. Please check again shortly.')}
              </p>
            ) : null}
            {rows.length ? (
              <table className="data-table">
                <thead>
                  <tr>
                    <th>{t('Rank')}</th>
                    <th>{t('Contributor')}</th>
                    <th>{t('Completed requests')}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr key={row.contributor.login}>
                      <td>{row.rank}</td>
                      <th>
                        <ExternalLink href={row.contributor.url}>
                          {row.contributor.login}
                        </ExternalLink>
                      </th>
                      <td>
                        {row.completed}{' '}
                        <span className="small">
                          {row.requests.map((number) => (
                            <span key={number}>
                              {' '}
                              <Link to={{ kind: 'formalizations', view: 'list', request: number }}>
                                #{number}
                              </Link>
                            </span>
                          ))}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : (
              <p>{t('No ranked formalization contributions yet.')}</p>
            )}
            <p>
              <Link to={{ kind: 'formalizations', view: 'list' }}>
                {t('Find a request to contribute to')}
              </Link>
            </p>
          </section>
        );
      }}
    </Async>
  );
}
