import { t } from '../i18n/locale';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { useSession } from '../auth/session';
import type { FeedbackView, KnowledgeDeclarationRef } from '../api/types';
import { Async, Field, InlineError } from '../components/ui';

export function FeedbackPage({ id, proposalKey }: { id: string; proposalKey: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.feedback(id, proposalKey, { signal }),
    [api, id, proposalKey],
  );
  const resource = useAsync(load);
  return (
    <div className="page">
      <h1>{t('Knowledge proposal')}</h1>
      <p>
        {t(
          'Inspect the exact evidence and license before authorizing export. A package download leaves upstream submission and admission to its existing process.',
        )}
      </p>
      <button className="button button-quiet" onClick={resource.reload}>
        {t('Refresh authorization and observations')}
      </button>
      <Async state={resource.state} onRetry={resource.reload}>
        {(view) => (
          <Proposal key={view.proposal.digest + view.state} view={view} changed={resource.reload} />
        )}
      </Async>
    </div>
  );
}
function Proposal({ view, changed }: { view: FeedbackView; changed: () => void }) {
  const api = useApi();
  const { person } = useSession();
  const p = view.proposal;
  const id = p.source.submission;
  const [consent, setConsent] = useState(false);
  const [error, setError] = useState<unknown>();
  const [busy, setBusy] = useState(false);
  const signer = !!person && p.required_signers.includes(person.id);
  const active = view.payload !== null;
  const approved = p.events.some((e) => e.event === 'authorized' && e.by === person?.id);
  async function action(work: () => Promise<unknown>) {
    setBusy(true);
    setError(undefined);
    try {
      await work();
      setConsent(false);
      changed();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  async function download() {
    const result = await api.exportFeedback(id, p.id);
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(result, null, 2)], { type: 'application/json' }),
    );
    const a = document.createElement('a');
    a.href = url;
    a.download = `mathesis-proposal-${p.digest.replace(':', '-')}.json`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  function revoke(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const reason = String(new FormData(e.currentTarget).get('reason'));
    void action(() => api.revokeFeedback(id, p.id, reason));
  }
  function observe(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const data = new FormData(e.currentTarget);
    void action(() =>
      api.feedbackReadback(id, p.id, {
        pull_request: Number(data.get('pull_request')),
        index_revision: String(data.get('index_revision')),
        observation_key: String(data.get('observation_key')),
        declarations: JSON.parse(String(data.get('declarations'))) as KnowledgeDeclarationRef[],
      }),
    );
  }
  return (
    <article className="panel">
      <h2>{p.kind.kind.replaceAll('_', ' ')}</h2>
      <p>
        {view.state.replaceAll('_', ' ')} · {p.source.record} {t(' · Source version ')}
        {p.source.version}
      </p>
      <p>
        {t('Exact digest: ')}
        <code>{p.digest}</code>
      </p>
      <p>
        {t('Producer: ')}
        {p.producer}
      </p>
      <h3>{t('Required authorization')}</h3>
      <ul>
        {p.required_signers.map((s) => (
          <li key={s}>
            {s}:{' '}
            {p.events.some((e) => e.event === 'authorized' && e.by === s)
              ? t('Exact package authorized')
              : t('Awaiting authorization')}
          </li>
        ))}
      </ul>
      {active ? (
        <>
          <h3>{t('Exact evidence and original attribution')}</h3>
          <pre className="code-block">{JSON.stringify(view.payload, null, 2)}</pre>
        </>
      ) : (
        <p>
          {t(
            'New package access is closed because consent or source evidence changed. Private event history is retained.',
          )}
        </p>
      )}
      {signer && active && !approved ? (
        <>
          <label>
            <input
              type="checkbox"
              checked={consent}
              onChange={(e) => setConsent(e.target.checked)}
            />
            {t(
              'I hold the applicable rights to my contributed material, have inspected this exact package and its original attribution and licenses, and authorize its export under Apache-2.0.',
            )}
          </label>
          <button
            className="button"
            disabled={busy || !consent}
            onClick={() =>
              void action(() =>
                api.authorizeFeedback(id, p.id, {
                  digest: p.digest,
                  license: 'Apache-2.0',
                  consent: 'feedback-export-2026-10-16',
                  rights_asserted: true,
                }),
              )
            }
          >
            {t('Authorize this exact package')}
          </button>
        </>
      ) : null}
      {signer && view.state === 'authorized' ? (
        <button className="button" disabled={busy} onClick={() => void action(download)}>
          {t('Download authorized proposal')}
        </button>
      ) : null}
      <InlineError error={error} />
      {signer && !p.events.some((e) => e.event === 'revoked') ? (
        <form onSubmit={revoke}>
          <p>
            {t(
              'Revocation stops new access, downloads and observations here. Previously issued licenses remain in effect.',
            )}
          </p>
          <Field label={t('Reason for revocation')} htmlFor="feedback-revoke">
            <textarea id="feedback-revoke" name="reason" required maxLength={2000} />
          </Field>
          <button className="button button-quiet" disabled={busy}>
            {t('Revoke further export')}
          </button>
        </form>
      ) : null}
      {person?.id === p.prepared_by &&
      view.state === 'authorized' &&
      p.events.some((e) => e.event === 'exported') ? (
        <details>
          <summary>{t('Observe an actual upstream publication')}</summary>
          <p>
            {t(
              'Enter a Trureturing PR and the full declaration identities from an independently admitted source index. This requests a read-only check; it does not assert acceptance. At most ten explicit observations are allowed.',
            )}
          </p>
          <form onSubmit={observe}>
            <Field label={t('Trureturing pull request number')} htmlFor="feedback-pr">
              <input
                id="feedback-pr"
                name="pull_request"
                type="number"
                min={1}
                max={100000000}
                required
              />
            </Field>
            <Field label={t('Admitted index revision')} htmlFor="feedback-index">
              <input
                id="feedback-index"
                name="index_revision"
                required
                pattern="sha256:[a-f0-9]{64}"
              />
            </Field>
            <Field
              label={t('Observation key (use a new key for a later check)')}
              htmlFor="feedback-key"
            >
              <input id="feedback-key" name="observation_key" required maxLength={100} />
            </Field>
            <Field
              label={t('Full declaration identities (JSON array)')}
              htmlFor="feedback-declarations"
            >
              <textarea id="feedback-declarations" name="declarations" required rows={8} />
            </Field>
            <button className="button" disabled={busy}>
              {t('Request read-only observation')}
            </button>
          </form>
        </details>
      ) : null}
      <h3>{t('Authorization and observation history')}</h3>
      {p.events.length === 0 ? <p>{t('No export authorizations recorded.')}</p> : null}
      {p.events.map((e, i) => (
        <section key={i}>
          {e.event === 'observed' ? (
            <>
              <h4>
                {t('PR #')}
                {e.observation.pull_request}: {e.observation.reference_state}
              </h4>
              <p>
                {t('Included in consumed source:')}{' '}
                {e.observation.merge_in_consumed_source ? t('Established') : t('Not established')}
              </p>
              <p>
                {t('Consumed source:')}{' '}
                <code>{e.observation.consumed_source_commit || 'Unavailable'}</code>
              </p>
              {e.observation.declarations.map((d, j) => (
                <p key={j}>
                  {d.identity.declaration_name_key}:{' '}
                  {d.present ? t('Exact declaration found') : t('Not established')}
                  {t('; module proposal provenance: ')}
                  {d.package_marker_present ? t('Found') : t('Not established')}
                  {t('; original freeze status: ')}
                  {d.freeze_status ?? 'Unavailable'}
                </p>
              ))}
              <ObservationEvidence id={id} proposal={p.id} request={e.observation.request} />
              <ul>
                {e.observation.limits.map((limit, j) => (
                  <li key={j}>{limit}</li>
                ))}
              </ul>
            </>
          ) : (
            <p>
              {e.event.replaceAll('_', ' ')} · {e.by} · {e.at}
            </p>
          )}
          <details>
            <summary>{t('Exact event record')}</summary>
            <pre className="code-block">{JSON.stringify(e, null, 2)}</pre>
          </details>
        </section>
      ))}
    </article>
  );
}

function ObservationEvidence({
  id,
  proposal,
  request,
}: {
  id: string;
  proposal: string;
  request: string;
}) {
  const api = useApi();
  const [evidence, setEvidence] = useState<Record<string, unknown>>();
  const [error, setError] = useState<unknown>();
  async function read() {
    try {
      setEvidence(await api.feedbackObservation(id, proposal, request));
    } catch (e) {
      setError(e);
    }
  }
  return (
    <details>
      <summary>{t('Saved observation evidence')}</summary>
      <button className="button button-quiet" onClick={() => void read()}>
        {t('Read checksummed observation')}
      </button>
      <InlineError error={error} />
      {evidence ? <pre className="code-block">{JSON.stringify(evidence, null, 2)}</pre> : null}
    </details>
  );
}
