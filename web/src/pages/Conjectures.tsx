import { t } from '../i18n/locale';
import { ResearchProblems } from './ResearchProblems';
import { InlineLatex } from '../components/Markdown';
import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { usePaged } from '../api/usePaged';
import type { ConjectureSummary } from '../api/types';
import { LatexText } from '../components/Markdown';
import { Pager } from '../components/Pager';
import { Async, Badge, DateText } from '../components/ui';
import { Link } from '../routing/router';
import { EntrantName } from './Leaderboard';
import { SUBMISSION_KIND_LABELS } from '../lib/labels';
import { AuthorLine } from '../components/AuthorLine';
export function ConjectureEntry({ conjecture: c }: { conjecture: ConjectureSummary }) {
  return (
    <article>
      <Link className="entry-title" to={{ kind: 'conjecture', record: c.record, claim: c.claim }}>
        <InlineLatex source={c.title} />
      </Link>
      <p className="entry-meta">
        <code>{c.record}</code>
        <span>{c.claim}</span>
        <span>{t(SUBMISSION_KIND_LABELS[c.kind ?? 'conjecture'])}</span>
        <span>{c.source}</span>
        <Badge tone={c.status === 'open' ? 'muted' : 'good'}>
          {c.status === 'solved'
            ? t('Solved')
            : c.status === 'disproved'
              ? t('Disproved')
              : t('Open')}
        </Badge>
      </p>
      <div className="entry-meta">
        {c.authors ? <AuthorLine authors={c.authors} /> : null}
        {c.accepted_at ? <DateText iso={c.accepted_at} /> : null}
      </div>
      <LatexText source={c.statement} className="conjecture-one-line" />
      <p className="small muted">
        {c.attempts} {t(' attempts ·')}{' '}
        {c.lean_statement_status === 'confirmed'
          ? t('Target confirmed by author')
          : t('Target awaiting author confirmation')}
      </p>
      {c.solver ? <EntrantName entrant={c.solver} /> : null}
    </article>
  );
}
export function ConjecturesPage() {
  const api = useApi();
  const [status, setStatus] = useState<'research' | 'open' | 'solved' | 'disproved'>('research');
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.listConjectures(
        { before, limit: 25, status: status === 'research' ? 'open' : status },
        { signal },
      ),
    [api, status],
  );
  const paged = usePaged(status === 'research' ? null : fetchPage);
  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t('Conjectures')}</h1>
          <p className="lede">
            {t('Follow a problem through new results, readable notes and formal proofs.')}
          </p>
        </div>
      </header>
      <div className="filter-row" aria-label={t('Status')}>
        {(['research', 'open', 'solved', 'disproved'] as const).map((v) => (
          <button
            key={v}
            type="button"
            className="filter-chip"
            aria-pressed={status === v}
            onClick={() => setStatus(v)}
          >
            {v === 'research'
              ? t('Research record')
              : v === 'open'
                ? t('Open questions')
                : v === 'solved'
                  ? t('Proved in Lean')
                  : t('Disproved in Lean')}
          </button>
        ))}
      </div>
      {status === 'research' ? (
        <ResearchProblems />
      ) : (
        <>
          <Async state={paged.state} onRetry={paged.reload}>
            {(listing) => {
              const items = listing.items.filter((c) => (c.status ?? 'open') === status);
              return items.length === 0 ? (
                <div className="empty-state">
                  <p>{t('No {status} conjectures here yet.', { status: t(status) })}</p>
                  <Link className="button" to={{ kind: 'submit', intake: 'conjecture' }}>
                    {t('Submit a conjecture')}
                  </Link>
                </div>
              ) : (
                <ul className="entry-list">
                  {items.map((c) => (
                    <li key={`${c.record}:${c.claim}`}>
                      <ConjectureEntry conjecture={c} />
                    </li>
                  ))}
                </ul>
              );
            }}
          </Async>
          <Pager paged={paged} />
        </>
      )}
    </div>
  );
}
