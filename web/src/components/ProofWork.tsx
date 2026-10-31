import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { CheckedContribution, FormalizationWork } from '../api/types';
import { useSession } from '../auth/session';
import { EntrantName } from '../pages/Leaderboard';
import { ProofAttribution } from './ProofAttribution';
import { DateText, InlineError } from './ui';

const labels: Record<FormalizationWork['state'], string> = {
  not_authorized: 'Automatic proof work awaits an author allowance.',
  paused: 'Automatic proof work is paused. Saved work is retained.',
  expired: 'The allowance has expired. Saved work is retained.',
  completed: 'The final target has an independently checked result.',
  allowance_used: 'The authorized allowance has been used. Saved work is retained.',
  active: 'Automatic proof work is authorized within the remaining allowance.',
};

/** Current goals, finite controls and checked auxiliary work share the existing result page. */
export function ProofWork({ record, claim }: { record: string; claim: string }) {
  const api = useApi();
  const { person } = useSession();
  const loadWork = useCallback(
    (signal: AbortSignal) => api.formalizationWork(record, claim, { signal }),
    [api, record, claim],
  );
  const loadContributions = useCallback(
    (signal: AbortSignal) => api.checkedContributions(record, claim, { signal }),
    [api, record, claim],
  );
  const work = useAsync(loadWork);
  const contributions = useAsync(loadContributions);
  if (!work.value) return null;
  return (
    <section className="proof-work" aria-label="Current proof work">
      <h2>Current proof work</h2>
      <p>The goal is a complete proof of the author-confirmed statement above.</p>
      {work.value.managed ? (
        <>
          <p>{labels[work.value.state]}</p>
          {work.value.state === 'not_authorized' && work.value.total_generations > 0 ? (
            <p>
              The confirmed statement changed. The saved allowance and used generations remain; the
              author must save an allowance for this statement before automatic work resumes.
            </p>
          ) : null}
          <p className="small">
            {work.value.used_generations} of {work.value.total_generations} authorized generations
            used
            {work.value.expires_at ? (
              <>
                {' '}
                · Expires <DateText iso={work.value.expires_at} withTime />
              </>
            ) : null}
            .
          </p>
          {work.value.can_control ? (
            <WorkControls
              key={work.value.target_digest}
              work={work.value}
              record={record}
              claim={claim}
              onChange={work.replace}
            />
          ) : null}
        </>
      ) : null}
      <h3>Checked propositions</h3>
      <p className="small">
        These named propositions passed the independent Lean verifier. Each receipt covers its
        stated proposition and complete source context. The final goal may remain unfinished.
      </p>
      {contributions.value?.length ? (
        <ul className="entry-list">
          {contributions.value.map((item) => (
            <Contribution key={item.origin.attempt} contribution={item} />
          ))}
        </ul>
      ) : contributions.state.status === 'error' ? (
        <p>Checked propositions are currently unavailable.</p>
      ) : (
        <p className="muted">No independently checked auxiliary propositions are available yet.</p>
      )}
      {person ? (
        <IntermediateForm record={record} claim={claim} target={work.value.target_digest} />
      ) : null}
      {person ? (
        <ReuseForm
          record={record}
          claim={claim}
          target={work.value.target_digest}
          available={contributions.value ?? []}
        />
      ) : null}
      <button
        type="button"
        className="button button-quiet"
        onClick={() => {
          work.reload();
          contributions.reload();
        }}
      >
        Refresh proof work
      </button>
    </section>
  );
}

function Contribution({ contribution: c }: { contribution: CheckedContribution }) {
  return (
    <li>
      <strong>{c.intermediate.title}</strong>
      <p>
        Proof contribution by <EntrantName entrant={c.contributor} /> · {c.intermediate.license}.
      </p>
      <p className="small">
        Mathematical source: {c.authors.map((a) => a.name).join(', ')}. Checked{' '}
        <DateText iso={c.receipt.checked_at} />.
      </p>
      <ProofAttribution attribution={c.attribution ?? undefined} />
      <details>
        <summary>Exact Lean proposition</summary>
        <pre>
          <code>{c.intermediate.proposition}</code>
        </pre>
      </details>
      <a href={`/api/v1/contributions/${encodeURIComponent(c.origin.attempt)}/bundle`}>
        Download proposition, sources and verification details
      </a>
    </li>
  );
}

function localDate(value: string) {
  const date = new Date(value);
  return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
}
function WorkControls({
  work,
  record,
  claim,
  onChange,
}: {
  work: FormalizationWork;
  record: string;
  claim: string;
  onChange: (work: FormalizationWork) => void;
}) {
  const api = useApi();
  const [total, setTotal] = useState(work.total_generations || 5);
  const [expires, setExpires] = useState(() =>
    localDate(work.expires_at ?? new Date(Date.now() + 86_400_000).toISOString()),
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const save = async (paused: boolean, retain: boolean) => {
    setBusy(true);
    setError(null);
    try {
      onChange(
        await api.controlFormalization(record, claim, {
          target_digest: work.target_digest,
          total_generations: retain ? work.total_generations : total,
          expires_at: retain && work.expires_at ? work.expires_at : new Date(expires).toISOString(),
          paused,
          change_allowance: !retain,
        }),
      );
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <details>
      <summary>Automatic proof work allowance</summary>
      <p>
        Allow 1–20 generations in total, expiring within 30 days. Pausing and resuming retain all
        used generations and the original work. Increasing the total or extending the expiry is a
        new explicit authorization.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void save(work.paused, false);
        }}
      >
        <label>
          Total generations
          <input
            type="number"
            min={Math.max(1, work.used_generations)}
            max={20}
            value={total}
            onChange={(e) => setTotal(e.target.valueAsNumber)}
            required
          />
        </label>
        <label>
          Expiry
          <input
            type="datetime-local"
            value={expires}
            onChange={(e) => setExpires(e.target.value)}
            required
          />
        </label>
        <button className="button" disabled={busy}>
          Save allowance
        </button>
        {work.total_generations > 0 && work.state !== 'not_authorized' ? (
          <button
            type="button"
            className="button button-quiet"
            disabled={busy || (!work.paused && work.state === 'expired')}
            onClick={() => void save(!work.paused, true)}
          >
            {work.paused ? 'Resume saved work' : 'Pause proof work'}
          </button>
        ) : null}
        <InlineError error={error} />
      </form>
    </details>
  );
}

function IntermediateForm({
  record,
  claim,
  target,
}: {
  record: string;
  claim: string;
  target: string;
}) {
  const api = useApi();
  const [error, setError] = useState<unknown>(null);
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const text = (key: string) => String(data.get(key) ?? '');
    setBusy(true);
    setError(null);
    setStatus('');
    try {
      await api.submitIntermediate(record, claim, {
        expected_target: target,
        intermediate: {
          title: text('title'),
          name: text('declaration'),
          proposition: text('proposition'),
          license: text('license'),
        },
        modules: { [text('module')]: text('source') },
      });
      setStatus(
        'Submitted for independent checking. Refresh proof work after verification. This does not complete the final goal.',
      );
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <details>
      <summary>Contribute an auxiliary proposition</summary>
      <p>
        Submit a named theorem in a self-contained Lean module using the pinned Mathlib environment.
        It can help later proof work even while the final goal remains open. Retain the original
        source license and credit; attributed existing projects can use the documented deposit
        interface.
      </p>
      <form onSubmit={(event) => void submit(event)}>
        <label>
          Proposition title
          <input name="title" maxLength={200} required />
        </label>
        <label>
          Fully qualified theorem name
          <input name="declaration" placeholder="Research.helper" required />
        </label>
        <label>
          Lean proposition
          <textarea name="proposition" maxLength={20000} required />
        </label>
        <label>
          Source license
          <input name="license" maxLength={200} required />
        </label>
        <label>
          Lean module name
          <input name="module" placeholder="Research.Helpers" required />
        </label>
        <label>
          Complete module source
          <textarea name="source" required />
        </label>
        <button className="button" disabled={busy}>
          Submit proposition for checking
        </button>
        <InlineError error={error} />
        {status ? <p role="status">{status}</p> : null}
      </form>
    </details>
  );
}

function ReuseForm({
  record,
  claim,
  target,
  available,
}: {
  record: string;
  claim: string;
  target: string;
  available: CheckedContribution[];
}) {
  const api = useApi();
  const [external, setExternal] = useState<CheckedContribution[]>([]);
  const [source, setSource] = useState('');
  const [selected, setSelected] = useState<string[]>([]);
  const [solution, setSolution] = useState('');
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');
  const choices = [...available, ...external].filter(
    (c, i, all) => all.findIndex((other) => other.origin.attempt === c.origin.attempt) === i,
  );
  const load = async () => {
    setError(null);
    try {
      const url = new URL(source, window.location.origin);
      const match = /^\/papers\/([^/]+)\/results\/([^/]+)\/?$/.exec(url.pathname);
      if (url.origin !== window.location.origin || !match)
        throw new Error('Enter a Mathesis result link.');
      setExternal(
        await api.checkedContributions(
          decodeURIComponent(match[1]!),
          decodeURIComponent(match[2]!),
        ),
      );
    } catch (e) {
      setExternal([]);
      setError(e);
    }
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    setBusy(true);
    setStatus('');
    try {
      await api.submitReusingAttempt(record, claim, {
        expected_target: target,
        solution,
        contributions: selected,
      });
      setStatus(
        'Submitted for independent checking with exact original sources and attribution. A checked-use link appears only if the verified proof term references the proposition.',
      );
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <details>
      <summary>Use checked propositions in a proof</summary>
      <p>
        Select up to eight propositions. Their exact modules, original licenses and contributor
        credit travel with your attempt. Import their modules in your proof. A joint source context
        does not establish that each proposition suffices on its own.
      </p>
      <label>
        Another Mathesis result link
        <input type="url" value={source} onChange={(e) => setSource(e.target.value)} />
      </label>
      <button type="button" className="button button-quiet" onClick={() => void load()}>
        Find checked propositions
      </button>
      <form onSubmit={(event) => void submit(event)}>
        {choices.map((c) => (
          <label className="checkbox" key={c.origin.attempt}>
            <input
              type="checkbox"
              checked={selected.includes(c.origin.attempt)}
              onChange={(e) =>
                setSelected((old) =>
                  e.target.checked
                    ? [...old, c.origin.attempt]
                    : old.filter((id) => id !== c.origin.attempt),
                )
              }
            />
            {c.intermediate.title} — {c.contributor.name} · {c.intermediate.license}
          </label>
        ))}
        <label>
          Complete Lean proof
          <textarea value={solution} onChange={(e) => setSolution(e.target.value)} required />
        </label>
        <button className="button" disabled={busy || !selected.length || selected.length > 8}>
          Submit proof with checked sources
        </button>
        <InlineError error={error} />
        {status ? <p role="status">{status}</p> : null}
      </form>
    </details>
  );
}
