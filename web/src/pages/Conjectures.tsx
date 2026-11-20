import { t } from '../i18n/locale';
import { ResearchProblems } from './ResearchProblems';
import { InlineLatex } from '../components/Markdown';
import { useCallback } from 'react';
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
        {c.authors ? <AuthorLine authors={c.authors} compact /> : null}
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
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.listConjectures({ before, limit: 25, status: 'open' }, { signal }),
    [api],
  );
  const paged = usePaged(fetchPage);
  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t('Conjectures')}</h1>
          <p className="lede">
            {t(
              'Explore collected open problems and conjectures, including questions with partial progress. Read the original statement and sources, or contribute a solution.',
            )}
          </p>
          <p>
            <Link to={{ kind: 'submit', intake: 'conjecture' }}>{t('Propose a conjecture')}</Link>
            {' · '}
            <Link to={{ kind: 'formalizations' }}>{t('Contribute a formalization')}</Link>
          </p>
        </div>
      </header>
      <ResearchProblems />
      <Async state={paged.state} onRetry={paged.reload}>
        {(listing) => {
          const items = listing.items.filter((c) => (c.status ?? 'open') === 'open');
          return items.length ? (
            <ul className="entry-list">
              {items.map((c) => (
                <li key={`${c.record}:${c.claim}`}>
                  <ConjectureEntry conjecture={c} />
                </li>
              ))}
            </ul>
          ) : null;
        }}
      </Async>
      <Pager paged={paged} />
    </div>
  );
}
