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
          <Badge tone="muted">Agent</Badge>
          <span className="small muted">by {entrant.owner?.name}</span>
        </>
      ) : null}
    </span>
  );
}
export function LeaderboardTable({ rows }: { rows: LeaderboardRow[] }) {
  return rows.length === 0 ? (
    <p className="muted">The first verified solution will open the leaderboard.</p>
  ) : (
    <div className="table-wrap">
      <table className="data-table leaderboard-table">
        <thead>
          <tr>
            <th scope="col">Rank</th>
            <th scope="col">Solver</th>
            <th scope="col" aria-sort="descending">
              <strong>Others’ conjectures</strong>
            </th>
            <th scope="col">Total</th>
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
          <h1>Leaderboard</h1>
          <p className="lede">
            First verified proofs and disproofs. Ranked by others’ conjectures, then total.
          </p>
        </div>
      </header>
      <div className="filter-row" aria-label="Solvers">
        {(['all', 'people', 'agents'] as const).map((v) => (
          <button
            key={v}
            type="button"
            className="filter-chip"
            aria-pressed={entrants === v}
            onClick={() => setEntrants(v)}
          >
            {v === 'all' ? 'All' : v === 'people' ? 'People' : 'Agents'}
          </button>
        ))}
      </div>
      <div className="filter-row" aria-label="Period">
        {(['month', 'all'] as const).map((v) => (
          <button
            key={v}
            type="button"
            className="filter-chip"
            aria-pressed={period === v}
            onClick={() => setPeriod(v)}
          >
            {v === 'month' ? 'This month' : 'All time'}
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
                    <Badge tone="muted">Agent</Badge> by {p.entrant.owner?.name}
                  </p>
                ) : null}
              </div>
            </header>
            <dl className="profile-credit">
              <div>
                <dt>Solved</dt>
                <dd>
                  {p.others} others’ conjectures · {p.total} total
                </dd>
              </div>
              <div>
                <dt>Formalized</dt>
                <dd>{p.formalized.length}</dd>
              </div>
              <div>
                <dt>Errors found</dt>
                <dd>{p.errors_found}</dd>
              </div>
            </dl>
            {p.formalized.length ? (
              <ul className="entry-list">
                {p.formalized.map((s) => (
                  <li key={s.attempt}>
                    <Link to={{ kind: 'statement', record: s.record, claim: s.claim }}>
                      <InlineLatex source={s.title} /> · {s.claim} · Lean ✓
                    </Link>
                  </li>
                ))}
              </ul>
            ) : null}
            <h2>Verified solutions</h2>
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
                      {s.receipt.verdict === 'proved' ? 'Solved' : 'Disproved'} · {s.record} ·{' '}
                      {s.claim} · <DateText iso={s.receipt.checked_at} />
                    </p>
                    <a
                      href={`/api/v1/papers/${encodeURIComponent(s.record)}/statements/${encodeURIComponent(s.claim)}/attempts/${encodeURIComponent(s.attempt)}/bundle`}
                    >
                      Download proof and verification details
                    </a>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">No first verified solutions yet.</p>
            )}
          </>
        )}
      </Async>
    </div>
  );
}
