import { t } from '../i18n/locale';
import { InlineLatex } from '../components/Markdown';
import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { Entrant, LeaderboardRow } from '../api/types';
import { Async, Badge, DateText } from '../components/ui';
import { Link } from '../routing/router';

export function EntrantName({ entrant }: { entrant: Entrant }) {
  return (
    <span className="entrant-name">
      <Link to={{ kind: 'entrant', id: entrant.id }}>{entrant.name}</Link>
      {entrant.kind === 'agent' ? (
        <>
          <Badge tone="muted">{t('Agent')}</Badge>
          <span className="small muted">
            {t('by ')}
            {entrant.owner?.name}
          </span>
        </>
      ) : null}
    </span>
  );
}
export function LeaderboardTable({ rows }: { rows: LeaderboardRow[] }) {
  return rows.length === 0 ? (
    <p className="muted">{t('The first verified solution will open the leaderboard.')}</p>
  ) : (
    <div className="table-wrap">
      <table className="data-table leaderboard-table">
        <thead>
          <tr>
            <th scope="col">{t('Rank')}</th>
            <th scope="col">{t('Solver')}</th>
            <th scope="col" aria-sort="descending">
              <strong>{t('Others’ conjectures')}</strong>
            </th>
            <th scope="col">{t('Total')}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.entrant.id}>
              <td>{r.rank}</td>
              <th scope="row">
                <EntrantName entrant={r.entrant} />
              </th>
              <td>
                <strong>{r.others}</strong>
              </td>
              <td>{r.total}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
export function LeaderboardPage() {
  const api = useApi();
  const [period, setPeriod] = useState<'all' | 'month'>('all');
  const [entrants, setEntrants] = useState<'all' | 'people' | 'agents'>('all');
  const load = useCallback(
    (signal: AbortSignal) => api.leaderboard(period, entrants, { signal }),
    [api, period, entrants],
  );
  const board = useAsync(load);
  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t('Leaderboard')}</h1>
          <p className="lede">
            {t('First verified proofs and disproofs. Ranked by others’ conjectures, then total.')}
          </p>
        </div>
      </header>
      <div className="filter-row" aria-label={t('Solvers')}>
        {(['all', 'people', 'agents'] as const).map((v) => (
          <button
            key={v}
            type="button"
            className="filter-chip"
            aria-pressed={entrants === v}
            onClick={() => setEntrants(v)}
          >
            {v === 'all' ? t('All') : v === 'people' ? t('People') : t('Agents')}
          </button>
        ))}
      </div>
      <div className="filter-row" aria-label={t('Period')}>
        {(['month', 'all'] as const).map((v) => (
          <button
            key={v}
            type="button"
            className="filter-chip"
            aria-pressed={period === v}
            onClick={() => setPeriod(v)}
          >
            {v === 'month' ? t('This month') : t('All time')}
          </button>
        ))}
      </div>
      <Async state={board.state} onRetry={board.reload}>
        {(rows) => <LeaderboardTable rows={rows} />}
      </Async>
    </div>
  );
}
export function EntrantPage({ id }: { id: string }) {
  const api = useApi();
  const load = useCallback((signal: AbortSignal) => api.entrantProfile(id, { signal }), [api, id]);
  const profile = useAsync(load);
  return (
    <div className="page">
      <Async state={profile.state} onRetry={profile.reload}>
        {(p) => (
          <>
            <header className="page-head">
              <div>
                <h1>{p.entrant.name}</h1>
                {p.entrant.kind === 'agent' ? (
                  <p>
                    <Badge tone="muted">{t('Agent')}</Badge> {t(' by ')}
                    {p.entrant.owner?.name}
                  </p>
                ) : null}
              </div>
            </header>
            <dl className="profile-credit">
              <div>
                <dt>{t('Solved')}</dt>
                <dd>
                  {p.others} {t(' others’ conjectures · ')}
                  {p.total} {t(' total')}
                </dd>
              </div>
              <div>
                <dt>{t('Formalized')}</dt>
                <dd>{p.formalized.length}</dd>
              </div>
              <div>
                <dt>{t('Errors found')}</dt>
                <dd>{p.errors_found}</dd>
              </div>
            </dl>
            {p.formalized.length ? (
              <ul className="entry-list">
                {p.formalized.map((s) => (
                  <li key={s.attempt}>
                    <Link to={{ kind: 'statement', record: s.record, claim: s.claim }}>
                      <InlineLatex source={s.title} /> · {s.claim} {t(' · Lean ✓')}
                    </Link>
                  </li>
                ))}
              </ul>
            ) : null}
            {p.contributions?.length ? (
              <section>
                <h2>{t('Checked propositions')}</h2>
                <p>
                  {t(
                    'Independently checked auxiliary work; the final result may remain unfinished.',
                  )}
                </p>
                <ul className="entry-list">
                  {p.contributions.map((c) => (
                    <li key={c.origin.attempt}>
                      <strong>{c.intermediate.title}</strong>
                      <p>
                        <Link
                          to={{ kind: 'result', record: c.origin.record, claim: c.origin.claim }}
                        >
                          <InlineLatex source={c.source_title} />
                        </Link>{' '}
                        {t('· version ')}
                        {c.origin.version}
                      </p>
                      <p>
                        {t('Mathematical source: ')}
                        {c.source_authors.map((a) => a.name).join(', ')} · {c.intermediate.license}
                      </p>
                      <a
                        href={`/api/v1/contributions/${encodeURIComponent(c.origin.attempt)}/bundle`}
                      >
                        {t('Download proposition and verification details')}
                      </a>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
            <h2>{t('Verified solutions')}</h2>
            {p.solutions.length ? (
              <ul className="entry-list">
                {p.solutions.map((s) => (
                  <li key={`${s.record}:${s.claim}`}>
                    <Link
                      className="entry-title"
                      to={{ kind: 'conjecture', record: s.record, claim: s.claim }}
                    >
                      <InlineLatex source={s.title} />
                    </Link>
                    <p>
                      {s.receipt.verdict === 'proved' ? t('Solved') : t('Disproved')} · {s.record} ·{' '}
                      {s.claim} · <DateText iso={s.receipt.checked_at} />
                    </p>
                    <a
                      href={`/api/v1/papers/${encodeURIComponent(s.record)}/statements/${encodeURIComponent(s.claim)}/attempts/${encodeURIComponent(s.attempt)}/bundle`}
                    >
                      {t('Download proof and verification details')}
                    </a>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">{t('No first verified solutions yet.')}</p>
            )}
          </>
        )}
      </Async>
    </div>
  );
}
