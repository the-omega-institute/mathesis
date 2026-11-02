import { t as tr } from '../../i18n/locale';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../../api/context';
import { useAsync } from '../../api/useAsync';
import type {
  ConjectureState,
  Decision,
  Person,
  PriorWork,
  ProofShape,
  SettlementOutcome,
  Submission,
  TaskGeneration,
} from '../../api/types';
import {
  buildConjectureState,
  buildJudgement,
  buildLiteratureReport,
  type LiteratureOutcome,
} from '../../lib/editorForms';
import {
  CONJECTURE_STATES,
  CONJECTURE_STATE_LABELS,
  RELATION_LABELS,
  SETTLEMENT_LABELS,
  SETTLEMENT_OUTCOMES,
  SHAPE_DESCRIPTIONS,
  SHAPE_LABELS,
  isOpenKind,
} from '../../lib/labels';
import { sourceLabel } from '../../lib/links';
import { TASK_KIND_LABELS, TASK_STATE_LABELS } from '../../lib/tasks';
import { Link } from '../../routing/router';
import { ConjectureStateBadge, ItemStateBadge } from '../badges';
import { ContributionList } from '../contributions';
import { ClaimRef, FormalArtifactView, statementTitle } from '../claims';
import { PriorWorkEditor } from '../PriorWorkEditor';
import { Async, Field, Fold, InlineError } from '../ui';

type OnChange = (s: Submission) => void;

function useMutation<T>(onDone: (value: T) => void) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const run = (call: () => Promise<T>, after?: () => void) => {
    setBusy(true);
    setError(null);
    call().then(
      (v) => {
        setBusy(false);
        onDone(v);
        after?.();
      },
      (e: unknown) => {
        setBusy(false);
        setError(e);
      },
    );
  };
  return { busy, error, setError, run };
}

/** Tools for editors (and admins); the server decides whether each call is allowed. */
export function EditorTools({
  submission,
  decision,
  person,
  onChange,
}: {
  submission: Submission;
  decision: Decision | null;
  person: Person | null;
  onChange: OnChange;
}) {
  const state = submission.status.state;
  return (
    <Fold id="editor-h" title={tr('Editor tools')} className="tools">
      {state === 'in_review' ? (
        <>
          <DecidePanel submission={submission} decision={decision} onChange={onChange} />
          <LiteratureReportForm submission={submission} onChange={onChange} />
          <JudgeForm submission={submission} />
          <AdoptPanel submission={submission} onChange={onChange} />
        </>
      ) : null}
      {state === 'in_review' || state === 'accepted' ? (
        <TasksPanel submission={submission} person={person} />
      ) : null}
      {state === 'accepted' ? (
        <>
          <FormalizationEditor submission={submission} onChange={onChange} />
          <ConjectureEditor submission={submission} onChange={onChange} />
        </>
      ) : null}
      {state === 'draft' ? (
        <p className="muted">{tr('The author has not confirmed the statements yet.')}</p>
      ) : null}
      {state === 'not_accepted' || state === 'withdrawn' ? (
        <p className="muted">{tr('No editorial action is open on this paper.')}</p>
      ) : null}
    </Fold>
  );
}

function DecidePanel({
  submission,
  decision,
  onChange,
}: {
  submission: Submission;
  decision: Decision | null;
  onChange: OnChange;
}) {
  const api = useApi();
  const m = useMutation(onChange);
  return (
    <div className="tool-form">
      <h3>{tr('Decision')}</h3>
      <p className="small">
        {tr('Applies the threshold to the current reports.')}{' '}
        {decision?.decision === 'pending'
          ? `The preview is pending (${decision.detail}); applying now will be refused.`
          : null}
      </p>
      <button
        type="button"
        className="button"
        disabled={m.busy}
        onClick={() => m.run(() => api.applyDecision(submission.id))}
      >
        {tr('Apply decision')}
      </button>
      <InlineError error={m.error} />
    </div>
  );
}

function LiteratureReportForm({
  submission,
  onChange,
}: {
  submission: Submission;
  onChange: OnChange;
}) {
  const api = useApi();
  const m = useMutation(onChange);
  const main = submission.claims.filter((c) => c.role === 'main' && !isOpenKind(c.kind));
  const targets = main.length > 0 ? main : submission.claims;
  const [outcome, setOutcome] = useState<LiteratureOutcome>('pass');
  const [summary, setSummary] = useState('');
  const [searched, setSearched] = useState('Reported sources\nzbMATH Open\narXiv');
  const [prior, setPrior] = useState<PriorWork[]>([]);
  const [knownBy, setKnownBy] = useState(0);
  const [question, setQuestion] = useState('');

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const built = buildLiteratureReport({ outcome, summary, searched, prior, knownBy, question });
    if (!built.ok) {
      m.setError(built.error);
      return;
    }
    m.run(() => api.fileReport(submission.id, 'literature', built.value));
  };
  const decisive = prior.map((p, i) => ({ p, i })).filter(({ p }) => p.relation !== 'related');

  return (
    <form className="tool-form" onSubmit={submit} aria-label={tr('File a literature report')}>
      <h3>{tr('S2 Literature report')}</h3>
      <p className="small">
        {tr(
          'The networked referee and Codex audit report opened literature sources in the S2 history above. This report decides the stage.',
        )}
      </p>
      <PriorWorkEditor idPrefix="lit-prior" value={prior} claims={targets} onChange={setPrior} />
      <Field label={tr('Searched (one per line)')} htmlFor="lit-searched">
        <textarea
          id="lit-searched"
          rows={3}
          value={searched}
          onChange={(e) => setSearched(e.target.value)}
        />
      </Field>
      <Field label={tr('Summary')} htmlFor="lit-summary">
        <textarea
          id="lit-summary"
          rows={3}
          value={summary}
          onChange={(e) => setSummary(e.target.value)}
        />
      </Field>
      <fieldset className="radio-list">
        <legend>{tr('Outcome')}</legend>
        <label className="radio">
          <input
            type="radio"
            name="lit-outcome"
            checked={outcome === 'pass'}
            onChange={() => setOutcome('pass')}
          />
          <span>{tr('Pass — no main result is stated in or directly implied by prior work')}</span>
        </label>
        <label className="radio">
          <input
            type="radio"
            name="lit-outcome"
            checked={outcome === 'fail'}
            onChange={() => setOutcome('fail')}
          />
          <span>{tr('Fail — a main result is known')}</span>
        </label>
        <label className="radio">
          <input
            type="radio"
            name="lit-outcome"
            checked={outcome === 'needs_human'}
            onChange={() => setOutcome('needs_human')}
          />
          <span>{tr('Needs another editor')}</span>
        </label>
      </fieldset>
      {outcome === 'fail' ? (
        <Field label={tr('Known by')} htmlFor="lit-known">
          <select
            id="lit-known"
            value={knownBy}
            onChange={(e) => setKnownBy(Number(e.target.value))}
          >
            {decisive.length === 0 ? (
              <option value={-1}>{tr('Add a prior work that states or implies it')}</option>
            ) : null}
            {decisive.map(({ p, i }) => (
              <option key={i} value={i}>
                {p.claim}: {sourceLabel(p.source)} ({tr(RELATION_LABELS[p.relation])})
              </option>
            ))}
          </select>
        </Field>
      ) : null}
      {outcome === 'needs_human' ? (
        <Field label={tr('Question')} htmlFor="lit-question">
          <input id="lit-question" value={question} onChange={(e) => setQuestion(e.target.value)} />
        </Field>
      ) : null}
      <button type="submit" className="button" disabled={m.busy}>
        {tr('File S2 report')}
      </button>
      <InlineError error={m.error} />
    </form>
  );
}

function JudgeForm({ submission }: { submission: Submission }) {
  const api = useApi();
  const proved = submission.claims.filter((c) => !isOpenKind(c.kind));
  const [claim, setClaim] = useState(proved[0]?.id ?? '');
  const [shape, setShape] = useState<ProofShape>('content');
  const [witnesses, setWitnesses] = useState('');
  const [rationale, setRationale] = useState('');
  const [done, setDone] = useState<string | null>(null);
  const m = useMutation<unknown>(() => setDone(claim));
  if (proved.length === 0) return null;
  const submit = (event: FormEvent) => {
    event.preventDefault();
    setDone(null);
    const built = buildJudgement(shape, witnesses, rationale);
    if (!built.ok) {
      m.setError(built.error);
      return;
    }
    m.run(
      () => api.judgeClaim(submission.id, claim, built.value),
      () => {
        setWitnesses('');
        setRationale('');
      },
    );
  };
  return (
    <form className="tool-form" onSubmit={submit} aria-label={tr('Judge a statement')}>
      <h3>{tr('S3 Judge a statement')}</h3>
      <p className="small">{tr('An editor’s judgement confirms the statement’s shape.')}</p>
      <Field label={tr('Statement')} htmlFor="judge-claim">
        <select id="judge-claim" value={claim} onChange={(e) => setClaim(e.target.value)}>
          {proved.map((c) => (
            <option key={c.id} value={c.id}>
              {c.id} · {statementTitle(c)}
              {c.role === 'main' ? tr(' (main)') : ''}
            </option>
          ))}
        </select>
      </Field>
      <fieldset className="radio-list">
        <legend>{tr('Shape')}</legend>
        {(['content', 'bind_only'] as const).map((s) => (
          <label key={s} className="radio">
            <input
              type="radio"
              name="judge-shape"
              checked={shape === s}
              onChange={() => setShape(s)}
            />
            <span>
              <strong>{tr(SHAPE_LABELS[s])}</strong> — {tr(SHAPE_DESCRIPTIONS[s])}
            </span>
          </label>
        ))}
      </fieldset>
      {shape === 'content' ? (
        <Field
          label={tr('Escape witnesses (one per line, Markdown with TeX)')}
          htmlFor="judge-witnesses"
        >
          <textarea
            id="judge-witnesses"
            rows={3}
            value={witnesses}
            onChange={(e) => setWitnesses(e.target.value)}
          />
        </Field>
      ) : null}
      <Field label={tr('Rationale')} htmlFor="judge-rationale">
        <textarea
          id="judge-rationale"
          rows={3}
          value={rationale}
          onChange={(e) => setRationale(e.target.value)}
        />
      </Field>
      <button type="submit" className="button" disabled={m.busy}>
        {tr('File judgement')}
      </button>
      {done ? (
        <p className="form-success small" role="status">
          {tr('Judgement on ')}
          {done} {tr(' filed.')}
        </p>
      ) : null}
      <InlineError error={m.error} />
    </form>
  );
}

function AdoptPanel({ submission, onChange }: { submission: Submission; onChange: OnChange }) {
  const api = useApi();
  const m = useMutation(onChange);
  return (
    <div className="tool-form">
      <h3>{tr('S3 Adopt judgements')}</h3>
      <p className="small">
        {tr(
          'Files the S3 report from the settled judgements (confirmed or corroborated). Every main result needs one.',
        )}
      </p>
      <button
        type="button"
        className="button button-quiet"
        disabled={m.busy}
        onClick={() => m.run(() => api.adoptJudgements(submission.id))}
      >
        {tr('Adopt judgements and file S3')}
      </button>
      <InlineError error={m.error} />
    </div>
  );
}

function TasksPanel({ submission, person }: { submission: Submission; person: Person | null }) {
  const api = useApi();
  const [generated, setGenerated] = useState<TaskGeneration | null>(null);
  const load = useCallback(
    (signal: AbortSignal) => api.listTasks({ submission: submission.id, limit: 100 }, { signal }),
    [api, submission.id],
  );
  const tasks = useAsync(load);
  const m = useMutation<TaskGeneration>((g) => {
    setGenerated(g);
    tasks.reload();
  });
  return (
    <div className="tool-form">
      <h3>{tr('Contributor tasks')}</h3>
      {submission.open_to_contributors ? (
        <>
          <p className="small">
            {tr(
              'Cuts tasks from the paper: escape judgements and literature checks while in review; formalizations of approved statements and conjecture probes after acceptance.',
            )}
          </p>
          <button
            type="button"
            className="button button-quiet"
            disabled={m.busy}
            onClick={() => m.run(() => api.generateTasks(submission.id))}
          >
            {tr('Generate tasks')}
          </button>
          {generated ? (
            <p className="form-success small" role="status">
              {generated.created} {tr(' created, ')}
              {generated.existing} {tr(' already open.')}
            </p>
          ) : null}
          <InlineError error={m.error} />
        </>
      ) : (
        <p className="small muted">
          {tr('The author has not opened this paper to volunteer contributors.')}
        </p>
      )}
      <Async state={tasks.state} onRetry={tasks.reload}>
        {(listing) =>
          listing.items.length === 0 ? (
            <p className="muted small">{tr('No tasks for this paper.')}</p>
          ) : (
            <ul className="entry-list">
              {listing.items.map((t) => (
                <li key={t.id}>
                  <p className="entry-meta">
                    <Link to={{ kind: 'task', id: t.id }}>{t.title}</Link>
                    <span>· {tr(TASK_KIND_LABELS[t.kind])}</span>
                    <span>· {tr(TASK_STATE_LABELS[t.status.state])}</span>
                  </p>
                  {t.status.state === 'submitted' || t.status.state === 'done' ? (
                    <details>
                      <summary>{tr('Contributions')}</summary>
                      <ContributionList task={t.id} person={person} />
                    </details>
                  ) : null}
                </li>
              ))}
            </ul>
          )
        }
      </Async>
    </div>
  );
}

function FormalizationEditor({
  submission,
  onChange,
}: {
  submission: Submission;
  onChange: OnChange;
}) {
  const api = useApi();
  const proposeM = useMutation(onChange);
  const proposable = submission.claims.filter(
    (c) => !isOpenKind(c.kind) && !submission.formalization.items.some((i) => i.claim === c.id),
  );
  const [claim, setClaim] = useState(proposable[0]?.id ?? '');
  const [reason, setReason] = useState('');
  return (
    <div className="tool-form">
      <h3>{tr('Formalization')}</h3>
      {proposable.length > 0 ? (
        <form
          className="inline-form"
          aria-label={tr('Propose a statement for formalization')}
          onSubmit={(e) => {
            e.preventDefault();
            if (reason.trim() === '') {
              proposeM.setError('Say why this statement is worth formalizing.');
              return;
            }
            proposeM.run(
              () => api.proposeFormalization(submission.id, claim, reason.trim()),
              () => setReason(''),
            );
          }}
        >
          <Field label={tr('Propose statement')} htmlFor="formal-claim">
            <select id="formal-claim" value={claim} onChange={(e) => setClaim(e.target.value)}>
              {proposable.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.id} · {statementTitle(c)}
                </option>
              ))}
            </select>
          </Field>
          <Field label={tr('Reason (shown to the author)')} htmlFor="formal-reason">
            <input id="formal-reason" value={reason} onChange={(e) => setReason(e.target.value)} />
          </Field>
          <button type="submit" className="button button-small" disabled={proposeM.busy}>
            {tr('Propose')}
          </button>
          <InlineError error={proposeM.error} />
        </form>
      ) : null}

      {submission.formalization.items.length > 0 ? (
        <ul className="formal-list">
          {submission.formalization.items.map((item) => (
            <li key={item.claim}>
              <div className="claim-head">
                <ClaimRef id={item.claim} claims={submission.claims} scope="ws" />
                <ItemStateBadge state={item.state} />
              </div>
              <p className="small">{item.reason}</p>
              {item.state.state === 'declined' ? (
                <p className="small muted">
                  {tr('Author declined: ')}
                  {item.state.reason}
                </p>
              ) : null}
              {item.state.state === 'verified' ? (
                <FormalArtifactView artifact={item.state.artifact} axioms={item.state.axioms} />
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

function ConjectureEditor({
  submission,
  onChange,
}: {
  submission: Submission;
  onChange: OnChange;
}) {
  const open = submission.claims.filter((c) => isOpenKind(c.kind));
  if (open.length === 0) return null;
  return (
    <div className="tool-form">
      <h3>{tr('Conjecture follow-ups')}</h3>
      <ul className="conjecture-list">
        {open.map((c) => {
          const current = submission.conjectures.find((f) => f.claim === c.id);
          return (
            <li key={c.id}>
              <div className="claim-head">
                <ClaimRef id={c.id} claims={submission.claims} scope="ws" />
                {current ? <ConjectureStateBadge state={current.state} /> : null}
              </div>
              <ConjectureForm
                submission={submission}
                claim={c.id}
                current={current?.state}
                onChange={onChange}
              />
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function ConjectureForm({
  submission,
  claim,
  current,
  onChange,
}: {
  submission: Submission;
  claim: string;
  current?: ConjectureState;
  onChange: OnChange;
}) {
  const api = useApi();
  const m = useMutation(onChange);
  const [state, setState] = useState<ConjectureState['state']>(current?.state ?? 'screening');
  const [reason, setReason] = useState('');
  const [outcome, setOutcome] = useState<SettlementOutcome>('proved');
  const [summary, setSummary] = useState('');
  const id = (k: string) => `conj-${claim}-${k}`;
  return (
    <form
      className="inline-form"
      aria-label={`Update follow-up of ${claim}`}
      onSubmit={(e) => {
        e.preventDefault();
        const built = buildConjectureState({ state, reason, outcome, summary });
        if (!built.ok) {
          m.setError(built.error);
          return;
        }
        m.run(() => api.updateConjecture(submission.id, claim, built.value));
      }}
    >
      <Field label={tr('State')} htmlFor={id('state')}>
        <select
          id={id('state')}
          value={state}
          onChange={(e) => setState(e.target.value as ConjectureState['state'])}
        >
          {CONJECTURE_STATES.map((s) => (
            <option key={s} value={s}>
              {tr(CONJECTURE_STATE_LABELS[s])}
            </option>
          ))}
        </select>
      </Field>
      {state === 'not_pursued' ? (
        <Field label={tr('Reason')} htmlFor={id('reason')}>
          <input id={id('reason')} value={reason} onChange={(e) => setReason(e.target.value)} />
        </Field>
      ) : null}
      {state === 'settled' ? (
        <>
          <Field label={tr('Outcome')} htmlFor={id('outcome')}>
            <select
              id={id('outcome')}
              value={outcome}
              onChange={(e) => setOutcome(e.target.value as SettlementOutcome)}
            >
              {SETTLEMENT_OUTCOMES.map((o) => (
                <option key={o} value={o}>
                  {tr(SETTLEMENT_LABELS[o])}
                </option>
              ))}
            </select>
          </Field>
          <Field label={tr('Summary')} htmlFor={id('summary')}>
            <input
              id={id('summary')}
              value={summary}
              onChange={(e) => setSummary(e.target.value)}
            />
          </Field>
        </>
      ) : null}
      <button type="submit" className="button button-small" disabled={m.busy}>
        {tr('Update')}
      </button>
      <InlineError error={m.error} />
    </form>
  );
}
