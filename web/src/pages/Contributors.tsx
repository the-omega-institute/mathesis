import { t } from '../i18n/locale';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { useSession } from '../auth/session';
import { Async } from '../components/ui';
import { formatCount } from '../lib/format';
import { shortId } from '../lib/labels';
import { Link } from '../routing/router';

export function ContributorsPage() {
  const api = useApi();
  const { person } = useSession();
  const load = useCallback(
    (signal: AbortSignal) => api.listContributors(undefined, { signal }),
    [api],
  );
  const credits = useAsync(load);
  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t('Contributors')}</h1>
          <p className="lede">
            {t(
              'What each contributor has submitted and what was verified. An escape judgement is verified when an editor confirms it or a judgement from a different model family and account agrees; a literature check or probe when an editor accepts it; a formalization when an editor records the verified Lean proof.',
            )}
          </p>
          <p className="muted small">
            {t(
              'Metered tokens were measured by NyxID on donated quota. Self-reported tokens are what contributors’ own agents reported. The two are kept apart and never added together.',
            )}{' '}
            <Link to={{ kind: 'contribute' }}>{t('How to contribute →')}</Link>
          </p>
        </div>
      </header>
      <Async state={credits.state} onRetry={credits.reload}>
        {(rows) =>
          rows.length === 0 ? (
            <p className="muted">{t('No contributions recorded yet.')}</p>
          ) : (
            <div className="table-wrap">
              <table className="data-table">
                <thead>
                  <tr>
                    <th scope="col">{t('Contributor')}</th>
                    <th scope="col" className="num">
                      {t('Verified')}
                    </th>
                    <th scope="col" className="num">
                      {t('Submitted')}
                    </th>
                    <th scope="col" className="num">
                      {t('Rejected')}
                    </th>
                    <th scope="col" className="num">
                      {t('Verified formalizations')}
                    </th>
                    <th scope="col" className="num">
                      {t('Metered tokens')}
                    </th>
                    <th scope="col" className="num">
                      {t('Self-reported tokens')}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((c) => (
                    <tr key={c.contributor}>
                      <th scope="row">
                        <code title={c.contributor}>{shortId(c.contributor)}</code>
                        {person?.id === c.contributor ? (
                          <span className="muted small"> {t(' (you)')}</span>
                        ) : null}
                      </th>
                      <td className="num">{formatCount(c.verified)}</td>
                      <td className="num">{formatCount(c.submitted)}</td>
                      <td className="num">{formatCount(c.rejected)}</td>
                      <td className="num">{formatCount(c.verified_formalizations)}</td>
                      <td className="num">{formatCount(c.metered_tokens)}</td>
                      <td className="num">{formatCount(c.reported_tokens)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )
        }
      </Async>
    </div>
  );
}
