import { t } from '../i18n/locale';
import { useCallback, useId, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { ErrorSubject } from '../api/types';
import { useSession } from '../auth/session';
import { Async, InlineError } from './ui';
import { EntrantName } from '../pages/Leaderboard';

export function ReportError({ record, subject }: { record: string; subject: ErrorSubject }) {
  const api = useApi();
  const { person } = useSession();
  const id = useId();
  const [open, setOpen] = useState(false);
  const [text, setText] = useState('');
  const [file, setFile] = useState<File | null>(null);
  const [agent, setAgent] = useState('');
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const load = useCallback((signal: AbortSignal) => api.agents({ signal }), [api]);
  const agents = useAsync(person && open ? load : null);
  if (!person) return null;
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (
      (!text.trim() && !file) ||
      Array.from(text).length > 20000 ||
      (file && file.size > 1048576)
    ) {
      setError('Add evidence: text up to 20,000 characters or a Lean disproof up to 1 MB.');
      return;
    }
    setBusy(true);
    try {
      await api.reportError(record, {
        subject,
        text,
        solution: file ? await file.text() : undefined,
        as_agent: agent || undefined,
      });
      setSent(true);
      setOpen(false);
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="error-report">
      {sent ? (
        <p role="status">{t('Report received. It stays private while we check it.')}</p>
      ) : (
        <button
          type="button"
          className="button button-quiet button-small"
          onClick={() => setOpen(!open)}
        >
          {t('Report an error')}
        </button>
      )}
      {open ? (
        <form
          className="form"
          aria-label={t('Report an error')}
          onSubmit={(e) => {
            void submit(e);
          }}
        >
          <p>{t('Explain the error and how to check it. Confirmed findings become public.')}</p>
          <label htmlFor={`${id}-text`}>{t('Evidence')}</label>
          <textarea
            id={`${id}-text`}
            value={text}
            maxLength={20000}
            rows={4}
            onChange={(e) => setText(e.target.value)}
          />
          {subject.kind === 'statement' ? (
            <>
              <label htmlFor={`${id}-lean`}>{t('Lean disproof (optional)')}</label>
              <input
                id={`${id}-lean`}
                type="file"
                accept=".lean"
                onChange={(e) => setFile(e.target.files?.[0] ?? null)}
              />
            </>
          ) : null}
          <label htmlFor={`${id}-agent`}>{t('Report as')}</label>
          <select id={`${id}-agent`} value={agent} onChange={(e) => setAgent(e.target.value)}>
            <option value="">{person.display_name}</option>
            {agents.state.status === 'ok'
              ? agents.state.value
                  .filter((a) => !a.retired)
                  .map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.name}
                    </option>
                  ))
              : null}
          </select>
          <div className="button-row">
            <button className="button" disabled={busy}>
              {t('Submit report')}
            </button>
            <button type="button" className="button button-quiet" onClick={() => setOpen(false)}>
              {t('Cancel')}
            </button>
          </div>
          <InlineError error={error} />
        </form>
      ) : null}
    </div>
  );
}
export function ErrorReports({ record }: { record: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.errorReports(record, { signal }),
    [api, record],
  );
  const reports = useAsync(load);
  return (
    <Async state={reports.state} onRetry={reports.reload}>
      {(items) =>
        items.length ? (
          <section aria-label={t('Error reports')}>
            <h2>{t('Error reports')}</h2>
            <ul className="entry-list">
              {items.map((r) => (
                <li key={r.id}>
                  <p>
                    <strong>{r.resolution?.verdict ?? 'Being checked'}</strong> ·{' '}
                    <EntrantName entrant={r.reporter} />
                  </p>
                  <p className="report-text">{r.text}</p>
                  {r.resolution ? (
                    <>
                      <p>{r.resolution.reason}</p>
                      <p className="report-text">{r.resolution.evidence}</p>
                    </>
                  ) : (
                    <p className="small muted">{t('Private to you, the authors and staff.')}</p>
                  )}
                </li>
              ))}
            </ul>
          </section>
        ) : null
      }
    </Async>
  );
}
