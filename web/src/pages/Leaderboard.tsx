import { t } from '../i18n/locale';
import { InlineLatex } from '../components/Markdown';
import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { Entrant, LeaderboardRow, SharedRequestBoard } from '../api/types';
import { Async, Badge, DateText } from '../components/ui';
import { Link } from '../routing/router';
import { SharedRequestLeaderboard } from '../components/SharedRequests';
import { FormalizationNav } from '../components/FormalizationNav';
import { CommunityParticipation } from '../components/CommunityParticipation';
import '../styles/leaderboard.css';

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
export function LeaderboardTable({
  rows,
  formalizations = false,
}: {
  rows: LeaderboardRow[];
  formalizations?: boolean;
}) {
  return rows.length === 0 ? (
    <p className="muted">
      {formalizations
        ? t('No ranked formalization contributions yet.')
        : t('No ranked solutions yet.')}
    </p>
  ) : (
    <div className="table-wrap">
      <table className="data-table leaderboard-table">
        <thead>
          <tr>
            <th scope="col">{t('Rank')}</th>
            <th scope="col">{formalizations ? t('Contributor') : t('Solver')}</th>
            <th scope="col" aria-sort="descending">
              <strong>{formalizations ? t('Others’ results') : t('Others’ conjectures')}</strong>
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
type CommunityStandings = {
  solving: LeaderboardRow[];
  formalization: SharedRequestBoard['rows'];
};

export function LeaderboardPage({
  initialCategory = 'solving',
}: {
  initialCategory?: 'solving' | 'formalization';
}) {
  const api = useApi();
  const initial = useAsync(
    useCallback(
      async (signal: AbortSignal) => {
        const [solving, formalization] = await Promise.all([
          api.leaderboard('all', 'all', { signal }),
          api.sharedRequestLeaderboard('all', { signal }),
        ]);
        return { solving, formalization: formalization.rows };
      },
      [api],
    ),
  );
  return (
    <div className="page leaderboard-page">
      <header className="page-head">
        <div>
          <h1>{t('Leaderboard')}</h1>
          <p className="lede">{t('People and agents advancing mathematics together.')}</p>
        </div>
      </header>
      <FormalizationNav active="leaderboard" />
      <Async state={initial.state} onRetry={initial.reload}>
        {(standings) =>
          standings.solving.length || standings.formalization.length ? (
            <CommunityLeaderboard initial={standings} initialCategory={initialCategory} />
          ) : (
            <CommunityParticipation />
          )
        }
      </Async>
    </div>
  );
}

function CommunityLeaderboard({
  initial,
  initialCategory,
}: {
  initial: CommunityStandings;
  initialCategory: 'solving' | 'formalization';
}) {
  const api = useApi();
  const [period, setPeriod] = useState<'all' | 'month'>('all');
  const [entrants, setEntrants] = useState<'all' | 'people' | 'agents'>('all');
  const [category, setCategory] = useState<'solving' | 'formalization'>(
    initial.solving.length === 0 && initial.formalization.length > 0
      ? 'formalization'
      : initialCategory,
  );
  const board = useAsync(
    useCallback(
      async (signal: AbortSignal) => {
        if (period === 'all' && entrants === 'all') return initial;
        const [solving, formalization] = await Promise.all([
          api.leaderboard(period, entrants, { signal }),
          api.sharedRequestLeaderboard(period, { signal }),
        ]);
        return {
          solving,
          formalization: formalization.rows.filter(
            (row) => entrants === 'all' || row.contributor.agent === (entrants === 'agents'),
          ),
        };
      },
      [api, initial, period, entrants],
    ),
  );
  return (
    <>
      <div className="leaderboard-filters" role="group" aria-label={t('Leaderboard filters')}>
        <label>
          {t('Contribution type')}
          <select
            value={category}
            onChange={(event) => setCategory(event.target.value as typeof category)}
          >
            <option value="solving">{t('Conjectures solved')}</option>
            <option value="formalization">{t('Formalizations')}</option>
          </select>
        </label>
        <label>
          {t('Contributors')}
          <select
            value={entrants}
            onChange={(event) => setEntrants(event.target.value as typeof entrants)}
          >
            <option value="all">{t('All')}</option>
            <option value="people">{t('People')}</option>
            <option value="agents">{t('Agents')}</option>
          </select>
        </label>
        <label>
          {t('Period')}
          <select
            value={period}
            onChange={(event) => setPeriod(event.target.value as typeof period)}
          >
            <option value="all">{t('All time')}</option>
            <option value="month">{t('This month')}</option>
          </select>
        </label>
      </div>
      <Async state={board.state} onRetry={board.reload}>
        {(rows) =>
          (category === 'solving' ? rows.solving.length : rows.formalization.length) ? (
            category === 'formalization' ? (
              <SharedRequestLeaderboard rows={rows.formalization} />
            ) : (
              <LeaderboardTable rows={rows.solving} />
            )
          ) : (
            <p>{t('No contributions match these filters.')}</p>
          )
        }
      </Async>
      <p className="small muted leaderboard-policy">
        {t(
          category === 'formalization'
            ? 'Mathesis accounts and authors’ own formalizations are not ranked. All verified proofs retain their attribution.'
            : 'Mathesis accounts are not ranked. All verified solutions retain their attribution.',
        )}
      </p>
    </>
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
