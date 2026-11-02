import { t as tr } from '../i18n/locale';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { ResearchCandidate, ResearchView, Submission } from '../api/types';
import { useSession } from '../auth/session';
import { Async, Field, InlineError, DateText } from './ui';
import { Markdown } from './Markdown';

const lines = (value: FormDataEntryValue | null) =>
  String(value ?? '')
    .split('\n')
    .map((s) => s.trim())
    .filter(Boolean);
const states: Record<string, string> = {
  awaiting_target: 'Preparing the exact target',
  consent_required: 'Current AI processing consent is required',
  target_budget_exhausted: 'Target preparation budget used',
  awaiting_confirmation: 'Review and confirm the target',
  target_mismatch: 'Target needs correction',
  target_rejected: 'Correction requested',
  ready: 'Ready for a proof attempt',
  verifying: 'Independent verification in progress',
  verified_target: 'The exact target was checked',
  budget_exhausted: 'Attempt budget used',
  expired: 'Authorization expired',
  stopped: 'Stopped by author',
  stale_source: 'Source or statements changed',
  resolved_by_audit: 'A later audit settled the original claim',
};
export function ResearchWorkspace({ submission }: { submission: Submission }) {
  const api = useApi();
  const { person } = useSession();
  const author = person?.id === submission.submitter;
  const load = useCallback(
    (signal: AbortSignal) => api.research(submission.id, { signal }),
    [api, submission.id],
  );
  const resource = useAsync(load);
  return (
    <details className="report">
      <summary>{tr('Private research goals')}</summary>
      <p>
        {tr(
          'Work on a missing hypothesis or bridge from the source audit. The submitting author authorizes a precise goal and confirms its Lean translation before any proof attempt.',
        )}
      </p>
      <button className="button button-quiet" onClick={resource.reload}>
        {tr('Refresh research status')}
      </button>
      <Async state={resource.state} onRetry={resource.reload}>
        {(view) => (
          <>
            {!view.candidates.length && !view.obligations.length ? (
              <p>{tr('No current audited research obligations are available.')}</p>
            ) : null}
            {author
              ? view.candidates.map((c) => (
                  <Adopt
                    key={c.digest}
                    candidate={c}
                    id={submission.id}
                    onChange={resource.replace}
                  />
                ))
              : null}
            {view.obligations.map((entry) => (
              <Obligation
                key={entry.obligation.id}
                entry={entry}
                id={submission.id}
                author={author}
                onChange={resource.replace}
              />
            ))}
          </>
        )}
      </Async>
    </details>
  );
}
function Adopt({
  candidate: c,
  id,
  onChange,
}: {
  candidate: ResearchCandidate;
  id: string;
  onChange: (v: ResearchView) => void;
}) {
  const api = useApi();
  const [error, setError] = useState<unknown>();
  const [busy, setBusy] = useState(false);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    if (data.get('consent') !== 'yes') return;
    setBusy(true);
    setError(undefined);
    try {
      onChange(
        await api.adoptResearch(id, {
          round: c.subject.round,
          claim: c.subject.claim,
          gap: c.gap,
          candidate_digest: c.digest,
          goal: {
            definitions: lines(data.get('definitions')),
            hypotheses: lines(data.get('hypotheses')),
            quantifiers: String(data.get('quantifiers')),
            conclusion: String(data.get('conclusion')),
            correspondence: String(data.get('correspondence')),
          },
          budget: {
            max_attempts: Number(data.get('attempts')),
            expires_at: new Date(Date.now() + Number(data.get('days')) * 86400000).toISOString(),
          },
          consent: 'research-obligation-2026-10-16',
        }),
      );
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  const prefix = c.digest.slice(7, 19);
  return (
    <details className="finding-card">
      <summary>
        {c.subject.claim} · {c.unresolved_text}
      </summary>
      <p>
        {tr('Source version ')}
        {c.subject.version} {tr(' · round ')}
        {c.subject.round}
      </p>
      <Markdown text={c.original_statement} />
      <h4>{tr('Premises and correspondence in the audit')}</h4>
      <Markdown text={c.binding.argument} />
      <ul>
        {c.binding.premises.map((p) => (
          <li key={p.statement_id}>
            <code>{p.module_path}</code> · {p.declaration_name_key}
          </li>
        ))}
      </ul>
      <ul>
        {c.binding.hypotheses.map((h, i) => (
          <li key={i}>
            {h.hypothesis}: {h.argument || 'Unresolved'}
          </li>
        ))}
      </ul>
      <ul>
        {c.binding.parameters.map((p, i) => (
          <li key={i}>
            {p.parameter} = {p.expression}
          </li>
        ))}
      </ul>
      <p>
        {tr(
          'Define the complete goal below, including all assumptions. Preparing a target sends this goal and the manuscript context to the configured AI services. Target preparation allows up to three versions, each with at most one automatic correction after independent inspection.',
        )}
      </p>
      <form onSubmit={(e) => void submit(e)}>
        <Field label={tr('Definitions (one per line)')} htmlFor={`${prefix}-definitions`}>
          <textarea id={`${prefix}-definitions`} name="definitions" maxLength={30000} />
        </Field>
        <Field label={tr('Assumptions (one per line)')} htmlFor={`${prefix}-hypotheses`}>
          <textarea id={`${prefix}-hypotheses`} name="hypotheses" maxLength={30000} />
        </Field>
        <Field label={tr('Variables and quantifiers')} htmlFor={`${prefix}-quantifiers`}>
          <textarea id={`${prefix}-quantifiers`} name="quantifiers" required maxLength={6000} />
        </Field>
        <Field label={tr('Exact conclusion to prove or refute')} htmlFor={`${prefix}-conclusion`}>
          <textarea id={`${prefix}-conclusion`} name="conclusion" required maxLength={6000} />
        </Field>
        <Field
          label={tr('How this goal addresses the audited gap')}
          htmlFor={`${prefix}-correspondence`}
        >
          <textarea
            id={`${prefix}-correspondence`}
            name="correspondence"
            required
            maxLength={6000}
          />
        </Field>
        <Field label={tr('Maximum proof attempts')} htmlFor={`${prefix}-attempts`}>
          <input
            id={`${prefix}-attempts`}
            name="attempts"
            type="number"
            min={1}
            max={5}
            defaultValue={2}
            required
          />
        </Field>
        <Field label={tr('Authorization expires in days')} htmlFor={`${prefix}-days`}>
          <input
            id={`${prefix}-days`}
            name="days"
            type="number"
            min={1}
            max={30}
            defaultValue={7}
            required
          />
        </Field>
        <label>
          <input type="checkbox" name="consent" value="yes" required />{' '}
          {tr(
            ' I authorize private research processing of this exact goal, including target preparation and independent AI inspection. I will confirm the exact target before submitting a proof.',
          )}
        </label>
        <InlineError error={error} />
        <button className="button" disabled={busy}>
          {busy ? tr('Authorizing…') : tr('Authorize private research')}
        </button>
      </form>
    </details>
  );
}
function Obligation({
  entry,
  id,
  author,
  onChange,
}: {
  entry: ResearchView['obligations'][number];
  id: string;
  author: boolean;
  onChange: (v: ResearchView) => void;
}) {
  const api = useApi();
  const [error, setError] = useState<unknown>();
  const [busy, setBusy] = useState(false);
  const [correction, setCorrection] = useState('');
  const [stopReason, setStopReason] = useState('');
  const [message, setMessage] = useState('');
  const [proof, setProof] = useState('');
  const { obligation: t, target } = entry;
  async function mutate(action: () => Promise<ResearchView>) {
    setBusy(true);
    setError(undefined);
    try {
      onChange(await action());
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!target) return;
    const data = new FormData(event.currentTarget);
    await mutate(() =>
      api.submitResearchAttempt(id, t.id, {
        target_digest: target.digest,
        solution: String(data.get('solution')),
        modules: JSON.parse(String(data.get('modules') || '{}')) as Record<string, string>,
      }),
    );
  }
  const closed = [
    'stale_source',
    'stopped',
    'expired',
    'resolved_by_audit',
    'consent_required',
  ].includes(entry.state);
  return (
    <section aria-label={tr('Authorized research goal')} className="finding-card">
      <h4>{states[entry.state] ?? entry.state}</h4>
      <Markdown text={t.goal.conclusion} />
      <p>
        {tr('Authorized ')}
        <DateText iso={t.authorized_at} /> {tr(' · expires')} <DateText iso={t.budget.expires_at} />{' '}
        · {entry.attempts.length}/{t.budget.max_attempts} {tr('attempts')}
      </p>
      <details>
        <summary>{tr('Complete goal and source binding')}</summary>
        <pre>{JSON.stringify({ goal: t.goal, candidate: t.candidate }, null, 2)}</pre>
      </details>
      {target ? (
        <>
          <Markdown text={target.reading} />
          <pre>{target.lean}</pre>
          <p>
            {tr('Target digest: ')}
            <code style={{ overflowWrap: 'anywhere' }}>{target.digest}</code>
          </p>
          <p>
            {tr('Toolchain: ')}
            {target.toolchain}
          </p>
          <p>
            {tr('Independent inspection: ')}
            {target.faithfulness?.verdict ?? 'Unavailable'} · {target.faithfulness?.reason}
          </p>
          {target.faithfulness?.mismatch ? <p>{target.faithfulness.mismatch}</p> : null}
          {author && !closed && target.response.state === 'awaiting_author' ? (
            <>
              <button
                className="button"
                disabled={busy || target.faithfulness?.verdict !== 'faithful'}
                onClick={() =>
                  void mutate(() =>
                    api.respondResearchTarget(id, t.id, { digest: target.digest, confirm: true }),
                  )
                }
              >
                {tr('Confirm this exact target')}
              </button>
              <Field label={tr('Target correction')} htmlFor={`${t.id}-correction`}>
                <textarea
                  id={`${t.id}-correction`}
                  value={correction}
                  onChange={(e) => setCorrection(e.target.value)}
                  maxLength={10000}
                />
              </Field>
              <button
                className="button button-quiet"
                disabled={busy || !correction.trim()}
                onClick={() =>
                  void mutate(() =>
                    api.respondResearchTarget(id, t.id, {
                      digest: target.digest,
                      confirm: false,
                      comment: correction,
                    }),
                  )
                }
              >
                {tr('Request target correction')}
              </button>
            </>
          ) : null}
          {author && entry.state === 'ready' ? (
            <form onSubmit={(e) => void submit(e)}>
              <Field label="Solution.lean" htmlFor={`${t.id}-solution`}>
                <textarea id={`${t.id}-solution`} name="solution" required maxLength={1048576} />
              </Field>
              <p>
                {tr(
                  'Import Target and provide wishpool_solution or wishpool_disproof for the confirmed proposition.',
                )}
              </p>
              <Field
                label={tr('Additional Lean modules (JSON object, optional)')}
                htmlFor={`${t.id}-modules`}
              >
                <textarea
                  id={`${t.id}-modules`}
                  name="modules"
                  placeholder={tr('{"MyLemma": "import Mathlib\n…"}')}
                />
              </Field>
              <button className="button" disabled={busy}>
                {tr('Submit for independent verification')}
              </button>
            </form>
          ) : null}
        </>
      ) : null}
      {entry.attempts.map((a) => (
        <div key={a.id}>
          <p>
            {tr('Attempt ')}
            {a.id}: {a.invalidated ?? a.receipt?.verdict ?? 'Awaiting verification'}
          </p>
          {a.receipt ? (
            <>
              <p>{a.receipt.reason}</p>
              <p>
                {tr('Checked ')}
                <DateText iso={a.receipt.checked_at} /> · {a.receipt.toolchain}
              </p>
            </>
          ) : null}
          <button
            className="button button-quiet"
            disabled={busy}
            onClick={() => {
              setBusy(true);
              setError(undefined);
              void api
                .researchAttempt(id, t.id, a.id)
                .then((p) => setProof(JSON.stringify(p, null, 2)), setError)
                .finally(() => setBusy(false));
            }}
          >
            {tr('Read saved sources and receipt')}
          </button>
          {author && !closed && a.receipt && a.receipt.verdict !== 'rejected' ? (
            <button
              className="button"
              disabled={busy}
              onClick={() => {
                setBusy(true);
                setError(undefined);
                void api
                  .recheckResearch(id, t.id, a.id)
                  .then(
                    () =>
                      setMessage(
                        'A fresh review has been requested to assess correspondence to the original claim.',
                      ),
                    setError,
                  )
                  .finally(() => setBusy(false));
              }}
            >
              {tr('Request review of this result')}
            </button>
          ) : null}
        </div>
      ))}
      {proof ? (
        <details open>
          <summary>{tr('Saved attempt')}</summary>
          <pre>{proof}</pre>
        </details>
      ) : null}
      <p className="hint">
        {tr(
          'A verified result applies to the exact target. A new source audit must establish how it addresses the original claim. Private research results do not receive public contribution credit.',
        )}
      </p>
      {author && !closed ? (
        <>
          <Field label={tr('Reason for stopping')} htmlFor={`${t.id}-stop`}>
            <input
              id={`${t.id}-stop`}
              value={stopReason}
              onChange={(e) => setStopReason(e.target.value)}
              maxLength={2000}
            />
          </Field>
          <button
            className="button button-quiet"
            disabled={busy || !stopReason.trim()}
            onClick={() => void mutate(() => api.stopResearch(id, t.id, stopReason))}
          >
            {tr('Stop research')}
          </button>
        </>
      ) : null}
      {message ? <p role="status">{message}</p> : null}
      <InlineError error={error} />
    </section>
  );
}
