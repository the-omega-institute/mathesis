import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async, DateText, ExternalLink } from './ui';
import { InlineLatex } from './Markdown';
import { t } from '../i18n/locale';

export const SHARED_REQUESTS = 'https://github.com/the-omega-institute/trureturing-requests/issues';

export function SharedRequests({ initialQuery = '' }: { initialQuery?: string }) {
  const api = useApi();
  const data = useAsync(
    useCallback((signal: AbortSignal) => api.sharedFormalizationRequests({ signal }), [api]),
  );
  const [query, setQuery] = useState(initialQuery);
  const [status, setStatus] = useState('open');
  const [limit, setLimit] = useState(30);
  return (
    <section aria-label={t('Shared formalization requests')}>
      <p>
        {t(
          'Choose a request, claim it in the shared discussion, and link your proof contribution there. People and agents use the same request.',
        )}
      </p>
      <p>
        {t(
          'To claim a request, assign yourself or comment /claim. Comment /unclaim to release your claim.',
        )}
      </p>
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
      <Async state={data.state} onRetry={data.reload}>
        {(snapshot) => {
          const rows = snapshot.requests.filter(
            (row) =>
              (status === 'all' ||
                (status === 'open'
                  ? ['open', 'claimed'].includes(row.state)
                  : row.state === status)) &&
              row.title.toLowerCase().includes(query.trim().toLowerCase()),
          );
          return (
            <>
              {snapshot.observed_at ? (
                <p className="small muted">
                  {t('Updated')} <DateText iso={snapshot.observed_at} />
                </p>
              ) : (
                <p>
                  {t(
                    'The shared request list is being synchronized. You can participate directly in the shared queue.',
                  )}
                </p>
              )}
              <ul className="entry-list">
                {rows.slice(0, limit).map((row) => (
                  <li key={row.number}>
                    <ExternalLink href={row.url}>
                      <InlineLatex source={row.title} />
                    </ExternalLink>
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
                    {row.completions.map((done) => (
                      <p key={done.url}>
                        <ExternalLink href={done.url}>
                          {t('Merged proof contribution')}
                        </ExternalLink>{' '}
                        ·{' '}
                        <ExternalLink href={done.contributor.url}>
                          {done.contributor.login}
                        </ExternalLink>
                      </p>
                    ))}
                    {row.paper_repository ? (
                      <p>
                        <ExternalLink href={row.paper_repository}>
                          {t('Paper repository')}
                        </ExternalLink>
                      </p>
                    ) : null}
                  </li>
                ))}
              </ul>
              {rows.length > limit ? (
                <button className="text-button" onClick={() => setLimit(limit + 30)}>
                  {t('Show more')}
                </button>
              ) : null}
              {snapshot.observed_at && !rows.length ? (
                <p>{t('No requests match these filters.')}</p>
              ) : null}
            </>
          );
        }}
      </Async>
      <p>
        <ExternalLink href={SHARED_REQUESTS}>{t('Open the shared request queue')}</ExternalLink>
      </p>
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
                {t(
                  'The shared request list is being synchronized. You can participate directly in the shared queue.',
                )}
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
                              <ExternalLink href={`${SHARED_REQUESTS}/${number}`}>
                                #{number}
                              </ExternalLink>
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
              <ExternalLink href={SHARED_REQUESTS}>
                {t('Find a request to contribute to')}
              </ExternalLink>
            </p>
          </section>
        );
      }}
    </Async>
  );
}
