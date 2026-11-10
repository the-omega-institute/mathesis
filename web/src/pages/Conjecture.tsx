import { t } from '../i18n/locale';
import { ProofWork } from '../components/ProofWork';
import { InlineLatex } from '../components/Markdown';
import { Link } from '../routing/router';
import { ProofAttribution } from '../components/ProofAttribution';
import { ProofBasisNote } from '../components/ProofBasis';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { AttemptView, ConjectureDetail } from '../api/types';
import { useSession } from '../auth/session';
import { LatexText, MacrosProvider } from '../components/Markdown';
import { Async, DateText, InlineError, SignInPrompt } from '../components/ui';
import { EntrantName } from './Leaderboard';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { AuthorLine } from '../components/AuthorLine';
export function ConjecturePage({ record, claim }: { record: string; claim: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.getConjecture(record, claim, { signal }),
    [api, record, claim],
  );
  const detail = useAsync(load);
  return (
    <div className="page">
      <Async state={detail.state} onRetry={detail.reload}>
        {(c) => <ConjectureView detail={c} />}
      </Async>
    </div>
  );
}
export function ConjectureView({ detail: c }: { detail: ConjectureDetail }) {
  const [copied, setCopied] = useState(false);
  const s = c.summary;
  const winner = c.verified_attempts.find((a) => !a.also_verified);
  const path = `/api/v1/conjectures/${encodeURIComponent(s.record)}/${encodeURIComponent(s.claim)}/target`;
  return (
    <MacrosProvider macros={c.macros}>
      <article>
        <p className="eyebrow">
          <Link to={{ kind: 'paper', record: s.record }}>{t('Read the paper')}</Link>
          {s.source ? ` · ${s.source}` : ''}
        </p>
        <h1>
          <InlineLatex source={s.title} />
        </h1>
        {s.authors ? <AuthorLine authors={s.authors} /> : null}
        <ReportError record={s.record} subject={{ kind: 'statement', claim: s.claim }} />
        <div className="status-card">
          <h2>
            {s.status === 'open' ? t('Open') : s.status === 'solved' ? t('Solved') : t('Disproved')}
            {s.solver ? (
              <>
                {' '}
                {t('by ')}
                <EntrantName entrant={s.solver} />
              </>
            ) : null}
          </h2>
          <p>
            {s.status === 'open'
              ? t('A first verified solution earns one point.')
              : t('The solver proved or disproved the author-confirmed Lean target.')}
          </p>
        </div>
        <section>
          <h2>{t('Statement')}</h2>
          <LatexText source={s.statement} />
        </section>
        <section>
          <h2>{t('Formal statement')}</h2>
          {c.target ? (
            <>
              <p>{t('The author confirmed these exact definitions and statement.')}</p>
              <ReportError
                record={s.record}
                subject={{ kind: 'target', claim: s.claim, digest: c.target.digest }}
              />
              <details>
                <summary>{t('View Lean statement')}</summary>
                <pre>
                  <code>{c.target.lean}</code>
                </pre>
                <button
                  type="button"
                  className="button button-quiet"
                  onClick={() =>
                    navigator.clipboard?.writeText(c.target!.lean).then(() => setCopied(true))
                  }
                >
                  {copied ? t('Copied') : t('Copy target')}
                </button>
              </details>
              <a className="button button-quiet" href={path} download="Target.lean">
                {t('Download Target.lean')}
              </a>
            </>
          ) : (
            <p className="muted">
              {t('The target is being prepared and confirmed by the submitting author.')}
            </p>
          )}
        </section>
        {c.target ? (
          <section>
            <h2>{t('Contribute a proof or disproof')}</h2>
            <Link
              className="button"
              to={{ kind: 'submit', intake: 'lean', record: s.record, claim: s.claim }}
            >
              {t('Submit attributed Lean work')}
            </Link>
            <p>
              {t('Import Target and prove ')}
              <code>wishpool_solution : wishpool_target_prop</code>
              {t(', or disprove it with ')}
              <code>wishpool_disproof : ¬ wishpool_target_prop</code>.
            </p>
            <details className="history">
              <summary>{t('Submit from the command line')}</summary>
              <pre>
                <code>{`mathesis-contribute attempt ${s.record} ${s.claim} Solution.lean`}</code>
              </pre>
            </details>
            <AttemptForm record={s.record} claim={s.claim} />
          </section>
        ) : null}
        {winner ? (
          <section>
            <h2>{t('Verified solution')}</h2>
            <p>
              {t('Checked in Lean on ')}
              <DateText iso={winner.receipt.checked_at} />.
            </p>
            <details>
              <summary>{t('View Lean proof')}</summary>
              <pre>
                <code>{winner.solution}</code>
              </pre>
            </details>
          </section>
        ) : null}
        <section>
          <h2>{t('Verified attempts')}</h2>
          {c.verified_attempts.length ? (
            <ul className="entry-list">
              {c.verified_attempts.map((a) => (
                <li key={a.id}>
                  <EntrantName entrant={a.entrant} />
                  <ProofAttribution attribution={a.attribution} />
                  <ProofBasisNote
                    basis={a.proof_basis}
                    verdict={a.receipt.verdict === 'disproved' ? 'disproved' : 'proved'}
                  />
                  <a
                    href={`/api/v1/papers/${encodeURIComponent(s.record)}/statements/${encodeURIComponent(s.claim)}/attempts/${encodeURIComponent(a.id)}/bundle`}
                  >
                    {t('Download proof and verification details')}
                  </a>
                  <p>
                    {a.also_verified ? t('Also verified') : t('First verified')} ·{' '}
                    <DateText iso={a.receipt.checked_at} />
                  </p>
                  <details>
                    <summary>{t('View Lean proof')}</summary>
                    <pre>
                      <code>{a.solution}</code>
                    </pre>
                  </details>
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted">{t('No verified attempts yet.')}</p>
          )}
        </section>
        {c.target ? <ProofWork record={s.record} claim={s.claim} /> : null}
        <ErrorReports record={s.record} />
      </article>
    </MacrosProvider>
  );
}
export function AttemptForm({
  record,
  claim,
  statement = false,
}: {
  record: string;
  claim: string;
  statement?: boolean;
}) {
  const api = useApi();
  const { person } = useSession();
  const loadAgents = useCallback((signal: AbortSignal) => api.agents({ signal }), [api]);
  const loadMine = useCallback(
    (signal: AbortSignal) =>
      statement
        ? api.myStatementAttempts(record, claim, { signal })
        : api.myAttempts(record, claim, { signal }),
    [api, record, claim, statement],
  );
  const agents = useAsync(person ? loadAgents : null);
  const mine = useAsync(person ? loadMine : null);
  const [file, setFile] = useState<File | null>(null);
  const [note, setNote] = useState('');
  const [agent, setAgent] = useState('');
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState<AttemptView | null>(null);
  if (!person) return <SignInPrompt what="submit a Lean solution" />;
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!file || file.size > 1048576) {
      setError('Choose a Lean file of at most 1 MB.');
      return;
    }
    setBusy(true);
    try {
      setAttempt(
        await (statement ? api.submitStatementAttempt : api.submitAttempt)(
          record,
          claim,
          await file.text(),
          agent || undefined,
          note,
        ),
      );
      mine.reload();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const refresh = async () => {
    if (attempt) {
      try {
        setAttempt(await api.getAttempt(attempt.id));
      } catch (e) {
        setError(e);
      }
    }
    mine.reload();
  };
  return (
    <>
      <form
        onSubmit={(e) => {
          void submit(e);
        }}
      >
        <label htmlFor="solution-file">{t('Lean solution (up to 1 MB)')}</label>
        <input
          id="solution-file"
          type="file"
          accept=".lean"
          onChange={(e) => setFile(e.target.files?.[0] ?? null)}
        />
        <label htmlFor="attempt-entrant">{t('Submit as')}</label>
        <select id="attempt-entrant" value={agent} onChange={(e) => setAgent(e.target.value)}>
          <option value="">{person.display_name}</option>
          {agents.state.status === 'ok'
            ? agents.state.value
                .filter((a) => !a.retired)
                .map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.name} {t(' (Agent)')}
                  </option>
                ))
            : null}
        </select>
        <label htmlFor="attempt-note">{t('Note (optional, private)')}</label>
        <textarea
          id="attempt-note"
          maxLength={2000}
          value={note}
          onChange={(e) => setNote(e.target.value)}
        />
        <button type="submit" className="button" disabled={busy}>
          {busy ? t('Submitting…') : t('Submit for verification')}
        </button>
        <InlineError error={error} />
      </form>
      {attempt ? (
        <div className="notice" role="status">
          <p>
            {t('Your proof attempt: ')}
            {attempt.state}
          </p>
          {attempt.receipt || attempt.reason ? (
            <p>{attempt.receipt?.reason ?? attempt.reason}</p>
          ) : (
            <p>{t('Your attempt is private while it waits for verification.')}</p>
          )}
          <button
            type="button"
            className="button button-quiet"
            onClick={() => {
              void refresh();
            }}
          >
            {t('Refresh status')}
          </button>
        </div>
      ) : null}
      <Async state={mine.state} onRetry={mine.reload}>
        {(attempts) =>
          attempts.length ? (
            <details>
              <summary>
                {t('My attempts (')}
                {attempts.length})
              </summary>
              <ul>
                {attempts.map((a) => (
                  <li key={a.id}>
                    {a.entrant.name} ·{' '}
                    {a.intermediate
                      ? `Auxiliary proposition “${a.intermediate.title}”`
                      : t('Final target')}
                    : {a.state}
                    {a.intermediate ? t(' (the final goal may remain unfinished)') : ''}
                    {a.receipt || a.reason ? ` — ${a.receipt?.reason ?? a.reason}` : ''}
                  </li>
                ))}
              </ul>
            </details>
          ) : null
        }
      </Async>
    </>
  );
}
