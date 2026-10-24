import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { CmaAttemptView } from '../api/types';
import { useSession } from '../auth/session';
import { CMA_ATTEMPT_STATUS_LABELS } from '../lib/labels';
import { Async, Field, InlineError, SignInPrompt } from './ui';

export const CMA_COPY =
  'Mathesis provides review during the early phase. Optionally connect a compute account to use your own resources for the source check and feedback steps.';

const STATUS_LABELS = {
  disconnected: 'Not connected',
  setup_needed: 'Setup needed',
  ready: 'Connected',
  unusable: 'Connection unavailable',
};
const REASONS: Record<string, string> = {
  consent_pending: 'Finish connecting through NyxID.',
  workspace_needed: 'Create a Compute workspace, then paste its ID below.',
  workspace_not_owned: 'Choose a workspace you own.',
  workspace_not_ready: 'Wait for your workspace to be ready, then recheck.',
  own_credentials_required: 'Set compute model access to own credentials.',
  workspace_setup_needed: 'Finish workspace setup in your compute account, then recheck.',
  authorization_refused: 'Reconnect to your compute account.',
  model_credentials_refused: 'Renew your model credentials in your compute account, then recheck.',
  author_quota_refused: 'Restore your model quota in your compute account, then recheck.',
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
                          Open compute setup
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
                        aria-label="Compute workspace"
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
                        <Field label="Compute workspace ID" htmlFor="cma-workspace">
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
  auth: 'Your compute authorization was refused.',
  setup: 'Your workspace setup was unavailable.',
  credential: 'Your model credentials were refused.',
  quota: 'Your model quota was unavailable.',
  cause_unknown: 'Your agents failed for an unknown reason.',
};

const STEP_LABELS = {
  audit: 'Source check',
  advice: 'Improvement suggestions',
  letter: 'Feedback letter',
  lean_probe: 'Lean check',
  conjecture_target: 'Conjecture target',
  research_target: 'Private research target',
  research_faithfulness: 'Research target inspection',
  problem_audit: 'Paper problem audit',
  target_faithfulness: 'Target faithfulness audit',
  error_report_audit: 'Error report audit',
  binding_audit: 'Independent binding inspection',
  venue_attempt: 'Venue agent attempt',
  legacy_unknown: 'Earlier review step',
};

export function CmaHistory({ attempts }: { attempts: CmaAttemptView[] }) {
  if (attempts.length === 0) return null;
  return (
    <section className="cma-history" aria-label="Who ran each step">
      <h2>Who ran each step</h2>
      <ul className="entry-list">
        {attempts.map((attempt, index) => (
          <li key={`${attempt.step}-${attempt.attempt}-${index}`}>
            <strong>
              {attempt.kind
                ? (STEP_LABELS[attempt.kind] ?? 'Other automated step')
                : 'Earlier review step'}
            </strong>{' '}
            · {attempt.payer === 'author' ? 'Your agents' : 'Mathesis'} ·{' '}
            {CMA_ATTEMPT_STATUS_LABELS[attempt.status] ?? 'Recorded'}
            {attempt.payer === 'operator' && ATTEMPT_REASONS[attempt.reason] ? (
              <p className="hint">{ATTEMPT_REASONS[attempt.reason]}</p>
            ) : null}
          </li>
        ))}
      </ul>
    </section>
  );
}
