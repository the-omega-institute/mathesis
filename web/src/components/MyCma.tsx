import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { CmaAttemptView } from '../api/types';
import { useSession } from '../auth/session';
import { Async, DateText, Field, InlineError, SignInPrompt } from './ui';

export const CMA_COPY =
  'Mathesis pays for review during the early phase; if you like, connect your CMA account to run the Codex steps for your submissions on your own agents.';

const STATUS_LABELS = {
  disconnected: 'Not connected',
  setup_needed: 'Setup needed',
  ready: 'Connected',
  unusable: 'Connection unavailable',
};
const REASONS: Record<string, string> = {
  consent_pending: 'Finish connecting through NyxID.',
  workspace_needed: 'Create a CMA workspace, then paste its ID below.',
  workspace_not_owned: 'Choose a workspace you own.',
  workspace_not_ready: 'Wait for your workspace to be ready, then recheck.',
  own_credentials_required: 'Set CMA model access to own credentials.',
  workspace_setup_needed: 'Finish workspace setup in CMA, then recheck.',
  authorization_refused: 'Reconnect to CMA.',
  model_credentials_refused: 'Renew your model credentials in CMA, then recheck.',
  author_quota_refused: 'Restore your model quota in CMA, then recheck.',
};

export function MyCma({ id }: { id?: string }) {
  const api = useApi();
  const { person } = useSession();
  const panel = useRef<HTMLElement>(null);
  useEffect(() => {
    if (id && window.location.hash === `#${id}`) panel.current?.scrollIntoView();
  }, [id]);
  const load = useCallback((signal: AbortSignal) => api.getCma({ signal }), [api]);
  const resource = useAsync(person ? load : null);
  const [workspace, setWorkspace] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const mutate = async (operation: () => ReturnType<typeof api.getCma>) => {
    setBusy(true);
    setError(null);
    try {
      resource.replace(await operation());
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const returnTo = window.location.pathname + window.location.search;
  return (
    <section ref={panel} id={id} aria-label="Use your own agents (optional)">
      <h2>Use your own agents (optional)</h2>
      <p>{CMA_COPY}</p>
      {!person ? (
        <SignInPrompt what="use your own agents" />
      ) : (
        <Async state={resource.state} onRetry={resource.reload}>
          {(status) => {
            const needsSetup =
              status.binding.status === 'setup_needed' || status.binding.status === 'unusable';
            return (
              <>
                <p role="status">
                  {status.enabled ? STATUS_LABELS[status.binding.status] : 'Not available yet'}
                </p>
                {status.enabled ? (
                  <>
                    {needsSetup && REASONS[status.binding.reason] ? (
                      <p className="hint">{REASONS[status.binding.reason]}</p>
                    ) : null}
                    <div className="button-row">
                      {status.binding.status !== 'ready' ? (
                        <a
                          className="button button-quiet"
                          href={`/auth/cma/connect?return_to=${encodeURIComponent(returnTo)}`}
                        >
                          Connect
                        </a>
                      ) : null}
                      {needsSetup ? (
                        <a
                          className="button button-quiet"
                          href={status.setup_url}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          Open CMA setup
                        </a>
                      ) : null}
                      {status.binding.status !== 'disconnected' ? (
                        <button
                          className="button button-quiet"
                          disabled={busy}
                          onClick={() => {
                            void mutate(() => api.disconnectCma());
                          }}
                        >
                          Disconnect
                        </button>
                      ) : null}
                    </div>
                    {needsSetup ? (
                      <form
                        className="inline-form"
                        aria-label="CMA workspace"
                        onSubmit={(event: FormEvent) => {
                          event.preventDefault();
                          const id = workspace.trim() || status.binding.workspace_id || '';
                          void mutate(() =>
                            api.setCmaWorkspace({
                              workspace_id: id,
                              expected_revision: status.binding.revision,
                            }),
                          );
                        }}
                      >
                        <Field label="CMA workspace ID" htmlFor="cma-workspace">
                          <input
                            id="cma-workspace"
                            value={workspace}
                            placeholder={status.binding.workspace_id ?? 'wks_…'}
                            maxLength={200}
                            onChange={(event) => setWorkspace(event.target.value)}
                          />
                        </Field>
                        <button
                          className="button button-quiet"
                          type="submit"
                          disabled={busy || !(workspace.trim() || status.binding.workspace_id)}
                        >
                          Save / recheck workspace
                        </button>
                      </form>
                    ) : null}
                  </>
                ) : null}
              </>
            );
          }}
        </Async>
      )}
      <InlineError error={error} />
    </section>
  );
}

const ATTEMPT_REASONS: Record<string, string> = {
  account: 'Your account was unavailable.',
  auth: 'Your CMA authorization was refused.',
  setup: 'Your workspace setup was unavailable.',
  credential: 'Your model credentials were refused.',
  quota: 'Your model quota was unavailable.',
  cause_unknown: 'Your agents failed for an unknown reason.',
};

const STEP_LABELS = {
  audit: 'Source check',
  advice: 'Advice',
  letter: 'Letter',
  lean_probe: 'Lean probe',
  conjecture_target: 'Conjecture target',
  research_target: 'Private research target',
  research_faithfulness: 'Research target inspection',
  problem_audit: 'Paper problem audit',
  target_faithfulness: 'Target faithfulness audit',
  error_report_audit: 'Error report audit',
  binding_audit: 'Independent binding inspection',
  venue_attempt: 'Venue agent attempt',
  legacy_unknown: 'Earlier CMA step',
};

export function CmaHistory({ attempts }: { attempts: CmaAttemptView[] }) {
  if (attempts.length === 0) return null;
  const groups = groupAttempts(attempts);
  return (
    <section className="cma-history" aria-label="Execution history">
      <h2>Execution history</h2>
      <p className="hint">
        Each row is one task for one review round and target. Retries are grouped under that step.
        Execution completed means the run finished; the overview shows whether its result was saved.
        A verified Lean proof is recorded separately.
      </p>
      <ul className="entry-list">
        {groups.map((group) => (
          <li key={group.key}>
            <div className="cma-history-head">
              <strong>{group.label}</strong>
              {group.round !== null ? <span className="muted">Round {group.round}</span> : null}
              <span className={`cma-history-status cma-history-${group.status}`}>
                {STATUS_LABELS_BY_ATTEMPT[group.status]}
              </span>
            </div>
            <p className="entry-meta">
              {group.payers.join(' · ')} · {group.attempts.length}{' '}
              {group.attempts.length === 1 ? 'attempt' : 'attempts'}
              {group.claim ? ` · ${group.claim}` : ''}
              {group.generation !== null ? ` · target revision ${group.generation}` : ''}
            </p>
            <AttemptTiming attempt={group.attempts.at(-1)} />
            {group.attempts.length === 1 ? (
              group.attempts[0]?.payer === 'operator' &&
              ATTEMPT_REASONS[group.attempts[0].reason] ? (
                <p className="hint">{ATTEMPT_REASONS[group.attempts[0].reason]}</p>
              ) : null
            ) : (
              <details className="history">
                <summary>Show attempts</summary>
                <ul>
                  {group.attempts.map((attempt) => (
                    <li key={`${attempt.step}-${attempt.attempt}-${attempt.admitted_at}`}>
                      Attempt {attempt.attempt + 1} ·{' '}
                      {attempt.payer === 'author' ? 'Your agents' : 'Mathesis'} ·{' '}
                      {STATUS_LABELS_BY_ATTEMPT[attempt.status]}
                      <AttemptTiming attempt={attempt} />
                      {attempt.payer === 'operator' && ATTEMPT_REASONS[attempt.reason] ? (
                        <p className="hint">{ATTEMPT_REASONS[attempt.reason]}</p>
                      ) : null}
                    </li>
                  ))}
                </ul>
              </details>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function AttemptTiming({ attempt }: { attempt?: CmaAttemptView }) {
  if (!attempt?.admitted_at && !attempt?.finished_at) return null;
  return (
    <p className="small muted">
      {attempt.admitted_at ? (
        <>
          Started <DateText iso={attempt.admitted_at} withTime />
        </>
      ) : null}
      {attempt.finished_at ? (
        <>
          {' '}
          · Finished <DateText iso={attempt.finished_at} withTime />
        </>
      ) : null}
    </p>
  );
}

const STATUS_LABELS_BY_ATTEMPT: Record<CmaAttemptView['status'], string> = {
  running: 'In progress',
  succeeded: 'Execution completed',
  failed: 'Failed',
};

type AttemptGroup = {
  key: string;
  label: string;
  round: number | null;
  claim: string | null;
  generation: number | null;
  payers: string[];
  status: CmaAttemptView['status'];
  attempts: CmaAttemptView[];
};

function groupAttempts(attempts: readonly CmaAttemptView[]): AttemptGroup[] {
  const groups = new Map<string, AttemptGroup>();
  for (const attempt of attempts) {
    const logicalStep = attempt.kind ?? attempt.step;
    const round = attempt.round ?? null;
    const claim = attempt.claim ?? null;
    const generation = attempt.target_generation ?? null;
    const key = `${round ?? attempt.step}:${logicalStep}:${claim ?? ''}:${generation ?? ''}`;
    const existing = groups.get(key);
    if (existing) {
      existing.attempts.push(attempt);
      const payer = attempt.payer === 'author' ? 'Your agents' : 'Mathesis';
      if (!existing.payers.includes(payer)) existing.payers.push(payer);
      existing.status = latestStatus(existing.attempts);
    } else {
      groups.set(key, {
        key,
        label: attempt.kind ? STEP_LABELS[attempt.kind] : attempt.step,
        round,
        claim,
        generation,
        payers: [attempt.payer === 'author' ? 'Your agents' : 'Mathesis'],
        status: attempt.status,
        attempts: [attempt],
      });
    }
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      attempts: group.attempts.sort(compareAttempts),
    }))
    .sort((a, b) => compareAttempts(b.attempts.at(-1)!, a.attempts.at(-1)!));
}

function latestStatus(attempts: readonly CmaAttemptView[]): CmaAttemptView['status'] {
  const latest = [...attempts].sort(compareAttempts).at(-1);
  return latest?.status ?? 'running';
}

function compareAttempts(a: CmaAttemptView, b: CmaAttemptView) {
  const aTime = a.admitted_at ? Date.parse(a.admitted_at) : 0;
  const bTime = b.admitted_at ? Date.parse(b.admitted_at) : 0;
  return aTime - bTime || a.attempt - b.attempt;
}
