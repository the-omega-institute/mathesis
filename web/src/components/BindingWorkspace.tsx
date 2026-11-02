import { t } from '../i18n/locale';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { useSession } from '../auth/session';
import type { BindingOption, PrivateBindingView, Submission } from '../api/types';
import { Async, Field, InlineError } from './ui';
import { Markdown } from './Markdown';
import { Link } from '../routing/router';

export function BindingWorkspace({ submission }: { submission: Submission }) {
  const api = useApi();
  const { person } = useSession();
  const author = person?.id === submission.submitter;
  const load = useCallback(
    (signal: AbortSignal) => api.privateBindings(submission.id, { signal }),
    [api, submission.id],
  );
  const resource = useAsync(load);
  const [error, setError] = useState<unknown>();
  const [busy, setBusy] = useState(false);
  async function revoke(event: FormEvent<HTMLFormElement>, key: string) {
    event.preventDefault();
    setBusy(true);
    setError(undefined);
    try {
      await api.revokeBinding(
        submission.id,
        key,
        String(new FormData(event.currentTarget).get('reason')),
      );
      resource.reload();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <details className="report">
      <summary>{t('Public binding discussions')}</summary>
      <p>
        {t(
          'For an accepted public paper with contributor access enabled, invite others to connect a precise statement to existing mathematics. You choose the statement, public context and source declarations before sharing. The manuscript and private review remain private.',
        )}
      </p>
      <button className="button button-quiet" onClick={resource.reload}>
        {t('Refresh binding discussions')}
      </button>
      <InlineError error={error} />
      <Async state={resource.state} onRetry={resource.reload}>
        {(view) => (
          <>
            {!view.options.length && !view.boards.length ? (
              <p>{t('No source-audited statements are ready to share.')}</p>
            ) : null}
            {author
              ? view.options.map((o) => (
                  <PublishSummary
                    key={o.digest}
                    option={o}
                    id={submission.id}
                    changed={resource.reload}
                  />
                ))
              : null}
            {view.boards.map((b) => (
              <section key={b.id}>
                <h3>
                  {b.target.subject.claim}
                  {t(': shared statement')}
                </h3>
                <Markdown text={b.target.statement} />
                <Markdown text={b.context} />
                <p>
                  {b.events.some((e) => e.event === 'revoked')
                    ? t('Sharing revoked')
                    : b.target.subject.version !== submission.versions.at(-1)?.number ||
                        b.target.subject.claims_revision !== submission.claims_revision
                      ? t('Source changed; sharing closed')
                      : t('Sharing authorized')}{' '}
                  · {b.license}
                </p>
                <Link to={{ kind: 'binding', id: submission.id, key: b.id }}>
                  {t('Open the public discussion')}
                </Link>
                <PrivateAudits board={b} />
                {author && !b.events.some((e) => e.event === 'revoked') ? (
                  <form onSubmit={(e) => void revoke(e, b.id)}>
                    <Field label={t('Reason for revoking sharing')} htmlFor={`revoke-${b.id}`}>
                      <textarea id={`revoke-${b.id}`} name="reason" required maxLength={2000} />
                    </Field>
                    <button disabled={busy} className="button button-quiet">
                      {t('Revoke sharing and stop further audits')}
                    </button>
                  </form>
                ) : null}
              </section>
            ))}
          </>
        )}
      </Async>
    </details>
  );
}
function PrivateAudits({ board }: { board: PrivateBindingView['boards'][number] }) {
  return (
    <details>
      <summary>{t('Private inspection history')}</summary>
      {board.events.map((event, i) =>
        event.event === 'audited' ? (
          <section key={i}>
            <strong>{event.audit.verdict}</strong>
            <Markdown text={event.audit.reason} />
            <Markdown text={event.audit.evidence} />
            <p>
              {event.audit.evidence_complete
                ? t('Required saved excerpts present')
                : t('Source evidence incomplete')}
            </p>
          </section>
        ) : null,
      )}
    </details>
  );
}
function PublishSummary({
  option: o,
  id,
  changed,
}: {
  option: BindingOption;
  id: string;
  changed: () => void;
}) {
  const api = useApi();
  const [context, setContext] = useState('');
  const [selected, setSelected] = useState<string[]>([]);
  const [consent, setConsent] = useState(false);
  const [error, setError] = useState<unknown>();
  const [busy, setBusy] = useState(false);
  const sources = o.target.sources.filter((s) => selected.includes(s.statement_id));
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!consent) return;
    setBusy(true);
    setError(undefined);
    try {
      await api.publishBinding(id, {
        option_digest: o.digest,
        sources,
        context,
        license: 'CC-BY-4.0',
        consent: 'binding-summary-2026-10-16',
      });
      setConsent(false);
      changed();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <details>
      <summary>
        {t('Share ')}
        {o.target.subject.claim} {t(' for a binding discussion')}
      </summary>
      <form onSubmit={(e) => void submit(e)}>
        <p>{t('Exact statement to publish:')}</p>
        <Markdown text={o.target.statement} />
        <p>
          {t('Submission ')}
          {id}
          {t(', source version ')}
          {o.target.subject.version}
          {t(', statements revision')} {o.target.subject.claims_revision}
          {t(', review round ')}
          {o.target.subject.round}.
        </p>
        <Field label={t('Public mathematical context')} htmlFor={`context-${o.digest}`}>
          <textarea
            id={`context-${o.digest}`}
            required
            maxLength={20000}
            value={context}
            onChange={(e) => {
              setContext(e.target.value);
              setConsent(false);
            }}
            rows={6}
          />
        </Field>
        <fieldset>
          <legend>{t('Source declarations to share (choose 1–40)')}</legend>
          {o.target.sources.map((s) => (
            <label key={s.statement_id}>
              <input
                type="checkbox"
                checked={selected.includes(s.statement_id)}
                onChange={(e) => {
                  setSelected(
                    e.target.checked
                      ? [...selected, s.statement_id]
                      : selected.filter((x) => x !== s.statement_id),
                  );
                  setConsent(false);
                }}
              />
              {s.declaration_name_key} — {s.module_path}
            </label>
          ))}
        </fieldset>
        <details>
          <summary>{t('Preview the complete public mathematical payload')}</summary>
          <pre className="code-block">
            {JSON.stringify(
              { target: { ...o.target, sources }, context, license: 'CC-BY-4.0' },
              null,
              2,
            )}
          </pre>
          <p>
            {t(
              'Your account attribution and authorization time are also public. Private queries and audit notes are excluded.',
            )}
          </p>
        </details>
        <label>
          <input type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} />
          {t(
            'I authorize public sharing of this exact statement, context and selected source identities under CC BY 4.0, with my attribution, and independent AI inspection of contributions against my manuscript.',
          )}
        </label>
        <p>
          {t(
            'Revocation closes new access and inspection here. People who already received material under CC BY 4.0 retain that license.',
          )}
        </p>
        <InlineError error={error} />
        <button
          className="button"
          disabled={busy || !consent || !sources.length || sources.length > 40}
        >
          {t('Publish this summary')}
        </button>
      </form>
    </details>
  );
}
