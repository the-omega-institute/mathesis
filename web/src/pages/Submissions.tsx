import { ProcessingStatus } from '../components/ProcessingStatus';
import { t } from '../i18n/locale';
import type { Submission } from '../api/types';
import { InlineLatex } from '../components/Markdown';
import { LeanWorkspace } from '../components/LeanDeposit';
import { useCallback, useState } from 'react';
import type { SubmissionScope } from '../api/client';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { usePaged } from '../api/usePaged';
import { hasRole, useSession } from '../auth/session';
import { SubmissionStatusBadge } from '../components/badges';
import { Pager } from '../components/Pager';
import { Async, DateText, InlineError, Loading, SignInPrompt } from '../components/ui';
import { formatAuthors, SUBMISSION_KIND_LABELS, newResults } from '../lib/labels';
import { Link } from '../routing/router';
import { awaitingDecision, decisionWaitingMessage } from '../lib/authorStanding';

type PageScope = 'mine' | 'queue';
const COPY: { [K in PageScope]: { title: string; lede: string; empty: string } } = {
  mine: {
    title: 'My work',
    lede: 'Your Lean contributions and research submissions. Track verification, read reviews, and return to work in progress.',
    empty: 'You have not submitted work yet.',
  },
  queue: {
    title: 'Review queue',
    lede: 'Current outcomes for all submissions. Open a record to inspect its review history.',
    empty: 'The queue is empty.',
  },
};

function newResultCount(s: import('../api/types').Submission): number {
  const reports = s.reports.filter((r) => r.claims_revision === s.claims_revision).reverse();
  const escape = reports.find((r) => r.payload.stage === 'escape')?.payload;
  const literature = reports.find((r) => r.payload.stage === 'literature')?.payload;
  if (escape?.stage !== 'escape') return 0;
  return s.claims.filter(
    (c) =>
      c.role === 'main' &&
      c.has_proof &&
      c.kind !== 'conjecture' &&
      c.kind !== 'question' &&
      !(literature?.stage === 'literature' ? literature.prior : []).some(
        (p) => p.claim === c.id && p.relation !== 'related',
      ) &&
      escape.assessments.some(
        (a) =>
          a.claim === c.id &&
          a.correctness === 'correct' &&
          a.shape === 'content' &&
          a.witnesses.length > 0,
      ),
  ).length;
}

export function SubmissionsPage({ scope }: { scope: PageScope }) {
  const { session, person } = useSession();
  const api = useApi();
  const [showRemoved, setShowRemoved] = useState(false);
  const listingScope: SubmissionScope = scope === 'mine' && showRemoved ? 'removed' : scope;
  const allowed =
    scope === 'mine' ? person !== null : hasRole(person, 'editor', 'reviewer', 'admin');
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.listSubmissions(listingScope, { before, limit: 20 }, { signal }),
    [api, listingScope],
  );
  const paged = usePaged(allowed ? fetchPage : null);
  const copy = COPY[scope];
  const loadNotifications = useCallback(
    (signal: AbortSignal) => api.notifications({ signal }),
    [api],
  );
  const loadErrors = useCallback(
    (signal: AbortSignal) => api.errorNotifications({ signal }),
    [api],
  );
  const errorNotifications = useAsync(scope === 'mine' && person ? loadErrors : null);
  const notifications = useAsync(scope === 'mine' && person ? loadNotifications : null);

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t(copy.title)}</h1>
          <p className="lede">{t(copy.lede)}</p>
        </div>
        {scope === 'mine' && person ? (
          <Link to={{ kind: 'submit' }} className="button">
            {t('Submit work')}
          </Link>
        ) : null}
      </header>
      {scope === 'queue' && hasRole(person, 'admin') ? <ProcessingStatus /> : null}
      <Async state={errorNotifications.state} onRetry={errorNotifications.reload}>
        {(reports) =>
          reports.length ? (
            <section className="notice">
              <h2>{t('Confirmed errors in your work')}</h2>
              <ul>
                {reports.map((r) => (
                  <li key={r.id}>
                    <Link to={{ kind: 'paper', record: r.record }}>{r.record}</Link>:{' '}
                    {r.resolution?.reason}
                  </li>
                ))}
              </ul>
            </section>
          ) : null
        }
      </Async>
      {session.status === 'loading' ? <Loading /> : null}
      {session.status !== 'loading' && person === null ? (
        <SignInPrompt what={`see ${copy.title.toLowerCase()}`} />
      ) : null}
      {person !== null && !allowed ? (
        <p className="notice notice-info">
          {t('The review queue is available to authorized staff.')}
        </p>
      ) : null}
      {notifications.state.status === 'ok' && notifications.state.value.length ? (
        <section aria-label={t('Conjecture solutions')}>
          <h2>{t('Your conjectures have answers')}</h2>
          <ul className="entry-list">
            {notifications.state.value.map((n) => (
              <li key={n.attempt}>
                <Link to={{ kind: 'conjecture', record: n.record, claim: n.claim }}>
                  {n.record} · {n.claim}
                </Link>
                <p>
                  {n.entrant.name} {t(' submitted a verified answer. ')}
                  <DateText iso={n.at} />
                </p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {scope === 'mine' && person ? (
        <>
          <LeanWorkspace />
          <h2>{t('Research submissions')}</h2>
          <p className="small">
            <button
              type="button"
              className="button button-quiet button-small"
              onClick={() => setShowRemoved((value) => !value)}
            >
              {t(showRemoved ? 'Back to my work' : 'Removed submissions')}
            </button>
          </p>
        </>
      ) : null}
      <Async state={paged.state} onRetry={paged.reload}>
        {(listing) =>
          listing.items.length === 0 ? (
            <p className="muted">
              {t(listingScope === 'removed' ? 'No removed submissions.' : copy.empty)}
            </p>
          ) : (
            <ul className="entry-list">
              {listing.items.map((s) => (
                <SubmissionRow key={s.id} submission={s} scope={scope} onChanged={paged.reload} />
              ))}
            </ul>
          )
        }
      </Async>
      <Pager paged={paged} />
    </div>
  );
}

function SubmissionRow({
  submission: s,
  scope,
  onChanged,
}: {
  submission: Submission;
  scope: PageScope;
  onChanged: () => void;
}) {
  const api = useApi();
  const { person } = useSession();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const removed = !!s.removed_from_work_at;
  const changeRemoval = async () => {
    setBusy(true);
    setError(null);
    try {
      await (removed ? api.restoreSubmission(s.id) : api.removeSubmission(s.id));
      onChanged();
    } catch (error) {
      setError(error);
    } finally {
      setBusy(false);
    }
  };
  const load = useCallback(
    (signal: AbortSignal) => api.reviewSummary(s.id, { signal }),
    [api, s.id],
  );
  const summary = useAsync(
    s.status.state !== 'draft' && s.status.state !== 'withdrawn' ? load : null,
  );
  const value = summary.value;
  const waitingForDecision = awaitingDecision(s, !!value?.delivered_at);
  const reportReady =
    value?.delivered_at && ['accepted', 'not_accepted', 'review_complete'].includes(s.status.state);
  const label =
    value?.waiting_since &&
    !value.interrupted &&
    !reportReady &&
    ['in_review', 'review_complete'].includes(s.status.state)
      ? 'Review temporarily delayed'
      : s.status.state === 'in_review'
        ? waitingForDecision
          ? 'Awaiting decision'
          : 'In review'
        : reportReady
          ? s.purpose === 'review_only'
            ? 'Report ready'
            : undefined
          : summary.state.status === 'loading'
            ? 'Loading review status…'
            : summary.state.status === 'error'
              ? 'Review status could not be loaded'
              : value?.interrupted
                ? 'Review interrupted'
                : value?.in_progress
                  ? 'Review in progress'
                  : undefined;
  return (
    <li>
      <Link
        to={{ kind: 'submission', id: s.id, ...(scope === 'queue' ? { staff: true } : {}) }}
        className="entry-title"
      >
        <InlineLatex source={s.title || s.versions.at(-1)?.filename || 'Untitled manuscript'} />
      </Link>
      <p className="entry-meta">
        <span>{t(SUBMISSION_KIND_LABELS[s.kind])}</span>
        <SubmissionStatusBadge status={s.status} label={label} />
        <span>{formatAuthors(s.authors)}</span>
        {s.status.state !== 'in_review' && newResultCount(s) > 0 ? (
          <span>{newResults(newResultCount(s))}</span>
        ) : null}
        {s.formalization.items.some((i) => i.state.state === 'verified') ? (
          <span>
            {t('Lean ✓ ')}
            {s.formalization.items.filter((i) => i.state.state === 'verified').length}
          </span>
        ) : null}
        <span>
          {t('· version ')}
          {s.versions.length}
        </span>
        <span>
          {t('· updated ')}
          <DateText iso={s.updated_at} />
        </span>
        {s.status.state === 'accepted' ? (
          <span>
            · <code>{s.status.record}</code>
          </span>
        ) : null}
      </p>
      {s.intake && !s.intake.confirmed_request && s.status.state === 'draft' ? (
        <p className="small">
          {t('Continue submission: check the PDF and complete the manuscript details.')}
        </p>
      ) : null}
      {reportReady ? (
        <p className="small">
          {t('The final report is ready to read.')}
          {value.has_execution_limits
            ? t(' The report explains which checks remain incomplete.')
            : ''}
        </p>
      ) : null}
      {waitingForDecision ? <p className="small">{decisionWaitingMessage(s)}</p> : null}
      {scope === 'mine' && person?.id === s.submitter && s.status.state === 'withdrawn' ? (
        <div className="small">
          <button
            type="button"
            className="button button-quiet button-small"
            disabled={busy}
            onClick={() => void changeRemoval()}
          >
            {t(removed ? 'Restore to my work' : 'Remove from my work')}
          </button>
          <InlineError error={error} />
        </div>
      ) : null}
      {summary.state.status === 'error' ? (
        <p className="small">
          {t('Your manuscript is saved.')}{' '}
          <button
            type="button"
            className="button button-quiet button-small"
            onClick={summary.reload}
          >
            {t('Retry status')}
          </button>
        </p>
      ) : null}
    </li>
  );
}
