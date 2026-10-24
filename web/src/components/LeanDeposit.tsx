import { InlineLatex } from './Markdown';
import { readLeanProject, type LeanProject } from '../lib/leanProject';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Link } from '../routing/router';
import { usePaged, type Paged } from '../api/usePaged';
import type {
  LeanDeposit,
  AttemptView,
  StatementDetail,
  LeanPreflight,
  NewLeanDeposit,
} from '../api/types';
import { useSession } from '../auth/session';
import { Async, Field, InlineError, DateText } from './ui';
import { Pager } from './Pager';

export function LeanDepositForm({
  targetRecord,
  targetClaim,
}: {
  targetRecord?: string;
  targetClaim?: string;
}) {
  const api = useApi();
  const load = useCallback(
    async (signal: AbortSignal) => {
      if (!targetRecord || !targetClaim) return null;
      const [statement, paper] = await Promise.all([
        api.getStatement(targetRecord, targetClaim, { signal }),
        api.getPaper(targetRecord, { signal }),
      ]);
      if (!statement.target)
        throw new Error(
          'This statement no longer has a confirmed target. Choose a current statement.',
        );
      return { statement, authors: paper.summary.authors.map((a) => a.name) };
    },
    [api, targetRecord, targetClaim],
  );
  const context = useAsync(load);
  return (
    <Async state={context.state} onRetry={context.reload}>
      {(data) => (
        <DepositForm
          key={data?.statement.target?.digest ?? 'standalone'}
          target={data?.statement}
          originalAuthors={data?.authors}
        />
      )}
    </Async>
  );
}
function DepositForm({
  target,
  originalAuthors,
}: {
  target?: StatementDetail;
  originalAuthors?: string[];
}) {
  const api = useApi();
  const { person } = useSession();
  const [title, setTitle] = useState(target ? `${target.title} — ${target.claim.label}` : '');
  const [authors, setAuthors] = useState<string[]>(() =>
    originalAuthors?.length ? originalAuthors : [person?.display_name ?? ''],
  );
  const [statement, setStatement] = useState(target?.claim.statement ?? '');
  const [reference, setReference] = useState(
    target ? `${window.location.origin}/papers/${encodeURIComponent(target.record)}` : '',
  );
  const [toolchain, setToolchain] = useState(target?.target?.toolchain ?? '');
  const [formalizers, setFormalizers] = useState<string[]>([person?.display_name ?? '']);
  const [sourceRevision, setSourceRevision] = useState('');
  const [supporting, setSupporting] = useState<File[]>([]);
  const [project, setProject] = useState<LeanProject | null>(null);
  const [preflight, setPreflight] = useState<LeanPreflight | null>(null);
  const [verification, setVerification] = useState<AttemptView | null>(null);
  const [license, setLicense] = useState('');
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [received, setReceived] = useState<LeanDeposit | null>(null);
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.formalizations({ before, limit: 10 }, { signal }),
    [api],
  );
  const deposits = usePaged(fetchPage);
  const importProject = async (files: File[]) => {
    setError(null);
    setPreflight(null);
    setBusy(true);
    try {
      const imported = await readLeanProject(files);
      setProject(imported);
      if (!target && imported.toolchain) setToolchain(imported.toolchain);
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const body = async (): Promise<NewLeanDeposit> => {
    if (!project && (!file || !file.name.endsWith('.lean') || !file.size))
      throw new Error('Choose a Lean source or complete project.');
    if (file && (file.size > 1048576 || !file.name.endsWith('.lean')))
      throw new Error('Solution.lean must be at most 1 MB.');
    const modules: Record<string, string> = Object.assign(
      Object.create(null) as Record<string, string>,
      project?.modules,
    );
    if (supporting.length + Object.keys(modules).length > 512)
      throw new Error('Use at most 512 supporting modules.');
    const existingBytes = Object.values(modules).reduce(
      (size, text) => size + new TextEncoder().encode(text).length,
      0,
    );
    if (
      supporting.reduce(
        (size, f) => size + f.size,
        existingBytes + (file?.size ?? new TextEncoder().encode(project?.solution ?? '').length),
      ) >
      8 * 1048576
    )
      throw new Error('Total Lean source must be at most 8 MB.');
    for (const module of supporting) {
      const name = module.name.replace(/\.lean$/, '');
      if (
        !module.name.endsWith('.lean') ||
        Object.keys(modules).some((m) => m.toLowerCase() === name.toLowerCase())
      )
        throw new Error(`Duplicate or invalid supporting module: ${name}`);
      modules[name] = await module.text();
    }
    return {
      modules,
      project_files: project?.project_files,
      formalizers: formalizers.map((a) => a.trim()).filter(Boolean),
      ...(sourceRevision.trim() ? { source_revision: sourceRevision.trim() } : {}),
      title,
      authors: authors.map((a) => a.trim()).filter(Boolean),
      statement,
      source_reference: { kind: 'url', locator: reference },
      toolchain,
      license,
      solution: file ? await file.text() : (project?.solution ?? ''),
    };
  };
  const check = async () => {
    setBusy(true);
    setError(null);
    try {
      setPreflight(await api.preflightFormalization(await body()));
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    setBusy(true);
    setVerification(null);
    try {
      const input = await body();
      const report = await api.preflightFormalization(input);
      setPreflight(report);
      const result = await api.submitFormalization(input);
      setReceived(result);
      deposits.reload();
      if (target?.target && report.errors.length === 0) {
        setVerification(
          await api.verifyFormalization(
            result.id,
            target.record,
            target.claim.id,
            target.target.digest,
          ),
        );
        deposits.reload();
      }
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <p className="lede">
        Submit an existing Lean formalization, with its original authors and research source.
      </p>
      {target?.target ? (
        <div className="notice notice-info">
          <strong>
            {target.claim.label} · <InlineLatex source={target.title} />
          </strong>
          <p>
            The author confirmed this exact target. Your submission will request independent
            verification against it.
          </p>
          <a
            href={`/api/v1/papers/${encodeURIComponent(target.record)}/statements/${encodeURIComponent(target.claim.id)}/target`}
            download="Target.lean"
          >
            Download Target.lean
          </a>
          <p className="small">Toolchain: {target.target.toolchain}</p>
        </div>
      ) : (
        <p className="hint">
          Your source stays in your private workspace until a linked proof is independently
          verified. <Link to={{ kind: 'formalizations' }}>Choose a confirmed statement</Link> to
          submit directly for verification.
        </p>
      )}
      <form
        className="form lean-deposit-form"
        aria-label="Submit Lean formalization"
        onChange={() => setPreflight(null)}
        onSubmit={(e) => {
          void submit(e);
        }}
      >
        <section className="submission-manuscript" aria-labelledby="lean-work-heading">
          <h2 id="lean-work-heading">Work & attribution</h2>
          <Field label="Title" htmlFor="lean-title">
            <input
              id="lean-title"
              required
              maxLength={1000}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </Field>
          <NameListField
            label="Original authors"
            itemLabel="Author"
            values={authors}
            onChange={setAuthors}
            required
            description={
              target
                ? 'Taken from the research record. Edit only if the attribution needs correction.'
                : 'Record the researchers credited on the original result.'
            }
          />
          <NameListField
            label="Formalization contributors"
            itemLabel="Contributor"
            values={formalizers}
            onChange={setFormalizers}
            description="Credit the people who wrote or maintained the Lean formalization."
            allowEmpty
          />
          <Field
            label="Git commit (optional)"
            htmlFor="lean-revision"
            hint="Full 40 or 64 character commit hash for the reported source."
          >
            <input
              id="lean-revision"
              maxLength={64}
              pattern="[a-fA-F0-9]{40}|[a-fA-F0-9]{64}"
              value={sourceRevision}
              onChange={(e) => setSourceRevision(e.target.value)}
            />
          </Field>
          <Field label="Mathematical statement" htmlFor="lean-statement">
            <textarea
              id="lean-statement"
              required
              maxLength={20000}
              rows={5}
              value={statement}
              onChange={(e) => setStatement(e.target.value)}
            />
          </Field>
          <Field label="Original research or source URL" htmlFor="lean-reference">
            <input
              id="lean-reference"
              type="url"
              required
              placeholder="Link to the paper or .lean source at a fixed revision"
              value={reference}
              onChange={(e) => setReference(e.target.value)}
            />
          </Field>
          <Field label="Lean / Mathlib version pin" htmlFor="lean-toolchain">
            <input
              id="lean-toolchain"
              required
              maxLength={200}
              placeholder="leanprover/lean4:v4.x, Mathlib <revision>"
              value={toolchain}
              onChange={(e) => setToolchain(e.target.value)}
            />
          </Field>
          <Field label="Source license" htmlFor="lean-license">
            <input
              id="lean-license"
              required
              maxLength={200}
              placeholder="e.g. Apache-2.0"
              value={license}
              onChange={(e) => setLicense(e.target.value)}
            />
          </Field>
        </section>
        <section className="submission-options" aria-labelledby="lean-files-heading">
          <h2 id="lean-files-heading">Source & verification</h2>
          <fieldset className="project-intake">
            <legend>Complete Lean project</legend>
            <p className="hint">
              Upload a source ZIP / tar.gz or choose a project folder. Up to 512 modules and 8 MB of
              Lean source; directory structure and pinned configuration are preserved. Build caches
              and .git are excluded from folder intake.
            </p>
            <Field label="Project archive" htmlFor="lean-project-archive">
              <input
                id="lean-project-archive"
                type="file"
                accept=".zip,.tar.gz,.tgz"
                disabled={busy}
                onChange={(e) => {
                  if (e.target.files?.length) void importProject(Array.from(e.target.files));
                }}
              />
            </Field>
            <Field label="Project folder" htmlFor="lean-project-folder">
              <input
                id="lean-project-folder"
                type="file"
                multiple
                ref={(node) => {
                  if (node) node.setAttribute('webkitdirectory', '');
                }}
                disabled={busy}
                onChange={(e) => {
                  if (e.target.files?.length) void importProject(Array.from(e.target.files));
                }}
              />
            </Field>
            {project ? (
              <div role="status">
                <p>
                  {Object.keys(project.modules).length} modules ·{' '}
                  {Object.keys(project.project_files).length} configuration / documentation files
                </p>
                <details>
                  <summary>Project files</summary>
                  <ul>
                    {Object.keys(project.project_files)
                      .concat(
                        Object.keys(project.modules).map(
                          (name) => `${name.replaceAll('.', '/')}.lean`,
                        ),
                      )
                      .map((path) => (
                        <li key={path}>
                          <code>{path}</code>
                        </li>
                      ))}
                  </ul>
                </details>
                <button
                  type="button"
                  className="button button-quiet button-small"
                  onClick={() => {
                    setProject(null);
                    setPreflight(null);
                  }}
                >
                  Remove project
                </button>
              </div>
            ) : null}
          </fieldset>
          <Field
            label="Solution.lean / standalone source (up to 1 MB)"
            htmlFor="lean-source"
            hint="Optional for a source deposit. To verify a project, supply an adapter importing Target and your project modules, with theorem wishpool_solution : wishpool_target_prop. Original project modules remain intact."
          >
            <input
              id="lean-source"
              type="file"
              accept=".lean"
              required={!project}
              onChange={(e) => {
                setPreflight(null);
                setFile(e.target.files?.[0] ?? null);
              }}
            />
          </Field>
          <Field
            label="Supporting Lean modules (optional)"
            htmlFor="lean-modules"
            hint="Up to 512 modules, 8 MB total. Choose a folder or archive above to preserve nested paths. Mathlib comes from the confirmed toolchain; custom dependencies and Lake scripts are not executed."
          >
            <input
              id="lean-modules"
              type="file"
              accept=".lean"
              multiple
              onChange={(e) => setSupporting(Array.from(e.target.files ?? []))}
            />
          </Field>
          <button
            type="button"
            className="button button-quiet"
            disabled={busy}
            onClick={() => {
              void check();
            }}
          >
            Run compatibility preflight
          </button>
          {preflight ? <PreflightReport report={preflight} /> : null}
          <InlineError error={error} />
          <button className="button" type="submit" disabled={busy}>
            {busy
              ? 'Submitting…'
              : target
                ? 'Submit and request verification'
                : 'Submit formalization'}
          </button>
        </section>
      </form>
      {received ? (
        <p className="notice notice-info" role="status">
          Source received: {received.title}.{' '}
          {verification ? `Verification ${verification.state}.` : 'Verification has not run yet.'}
        </p>
      ) : null}
      <DepositListing deposits={deposits} target={target} />
    </>
  );
}

function NameListField({
  label,
  itemLabel,
  values,
  onChange,
  description,
  required = false,
  allowEmpty = false,
}: {
  label: string;
  itemLabel: string;
  values: string[];
  onChange: (values: string[]) => void;
  description: string;
  required?: boolean;
  allowEmpty?: boolean;
}) {
  const fieldId = label.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-');
  return (
    <fieldset className="name-list-field">
      <legend>{label}</legend>
      <p className="hint">{description}</p>
      <div className="name-list">
        {values.map((value, index) => {
          const id = `${fieldId}-${index}`;
          return (
            <div className="name-list-row" key={id}>
              <label htmlFor={id}>
                {itemLabel} {index + 1}
              </label>
              <input
                id={id}
                required={required}
                maxLength={200}
                value={value}
                placeholder={index === 0 ? 'Full name as it should appear in the record' : ''}
                onChange={(event) =>
                  onChange(values.map((entry, i) => (i === index ? event.target.value : entry)))
                }
              />
              {allowEmpty || values.length > 1 ? (
                <button
                  type="button"
                  className="button button-quiet button-small"
                  onClick={() => onChange(values.filter((_, i) => i !== index))}
                >
                  Remove
                </button>
              ) : null}
            </div>
          );
        })}
      </div>
      <button
        type="button"
        className="button button-quiet button-small"
        onClick={() => onChange([...values, ''])}
      >
        Add {itemLabel.toLowerCase()}
      </button>
    </fieldset>
  );
}

export function LeanWorkspace() {
  const api = useApi();
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.formalizations({ before, limit: 10 }, { signal }),
    [api],
  );
  const deposits = usePaged(fetchPage);
  return <DepositListing deposits={deposits} />;
}
function DepositListing({
  deposits,
  target,
}: {
  deposits: Paged<LeanDeposit>;
  target?: StatementDetail;
}) {
  return (
    <section>
      <h2>Your formalization submissions</h2>
      <Async state={deposits.state} onRetry={deposits.reload}>
        {(page) =>
          page.items.length ? (
            <ul className="entry-list">
              {page.items.map((d) => (
                <li key={d.id}>
                  <DepositEntry deposit={d} onChange={deposits.reload} target={target} />
                </li>
              ))}
            </ul>
          ) : (
            <p className="muted">Your submitted Lean sources will appear here.</p>
          )
        }
      </Async>
      <Pager paged={deposits} />
    </section>
  );
}
function DepositEntry({
  deposit,
  onChange,
  target,
}: {
  deposit: LeanDeposit;
  onChange: () => void;
  target?: StatementDetail;
}) {
  const api = useApi();
  const [record, setRecord] = useState(target?.record ?? '');
  const [claim, setClaim] = useState(target?.claim.id ?? '');
  const [adapter, setAdapter] = useState<File | null>(null);
  const [result, setResult] = useState<AttemptView | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const run = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const current = await api.getStatement(record.trim(), claim.trim());
      if (!current.target) throw new Error('The author has not confirmed a target.');
      if (target?.target && current.target.digest !== target.target.digest)
        throw new Error('Target changed. Reopen the statement before verification.');
      setResult(
        await api.verifyFormalization(
          deposit.id,
          record.trim(),
          claim.trim(),
          current.target.digest,
          adapter ? await adapter.text() : undefined,
        ),
      );
      onChange();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const refresh = async (id: string) => {
    try {
      setResult(await api.getAttempt(id));
    } catch (e) {
      setError(e);
    }
  };
  return (
    <article>
      <h3>
        <InlineLatex source={deposit.title} />
      </h3>
      <p className="small muted">
        {deposit.authors.join(', ')} · received <DateText iso={deposit.submitted_at} />
      </p>
      <p>
        <a href={`/api/v1/formalizations/${encodeURIComponent(deposit.id)}/source`}>
          Download submitted source
        </a>{' '}
        · {deposit.license} ·{' '}
        <a href={`/api/v1/formalizations/${encodeURIComponent(deposit.id)}/project`}>
          Download complete project
        </a>
        {deposit.modules && Object.keys(deposit.modules).length
          ? ` · ${Object.keys(deposit.modules).length} supporting modules`
          : ''}
      </p>
      <details>
        <summary>Verify against a confirmed statement</summary>
        <p>
          <Link to={{ kind: 'formalizations' }}>Browse confirmed statements →</Link>
        </p>
        <p className="hint">
          Use a source module that imports Target and declares wishpool_solution or
          wishpool_disproof. The stored toolchain must match the target. Verified proofs and their
          original attribution appear on the public source record.
        </p>
        <form
          className="form"
          onSubmit={(e) => {
            void run(e);
          }}
        >
          <label>
            Paper record
            <input
              required
              placeholder="WP-2026-0001"
              value={record}
              onChange={(e) => setRecord(e.target.value)}
            />
          </label>
          <label>
            Statement
            <input
              required
              placeholder="C1"
              value={claim}
              onChange={(e) => setClaim(e.target.value)}
            />
          </label>
          <Field
            label="Solution.lean adapter (optional replacement)"
            htmlFor={`adapter-${deposit.id}`}
            hint="Use the saved source, or attach a target adapter while preserving the deposited project."
          >
            <input
              id={`adapter-${deposit.id}`}
              type="file"
              accept=".lean"
              onChange={(e) => setAdapter(e.target.files?.[0] ?? null)}
            />
          </Field>
          <button className="button button-small" disabled={busy}>
            {busy ? 'Submitting…' : 'Request verification'}
          </button>
        </form>
      </details>
      {deposit.attempts.map((id) => (
        <button
          key={id}
          type="button"
          className="button button-quiet button-small"
          onClick={() => {
            void refresh(id);
          }}
        >
          Check verification {id.slice(0, 8)}
        </button>
      ))}
      {result ? (
        <p role="status">
          Verification: <strong>{result.state}</strong>
          {result.receipt
            ? ` — ${result.receipt.reason}`
            : '. Waiting for an independent verifier receipt.'}
        </p>
      ) : null}
      <InlineError error={error} />
    </article>
  );
}

function PreflightReport({ report }: { report: LeanPreflight }) {
  return (
    <section
      className={`notice ${report.errors.length ? 'notice-error' : 'notice-info'}`}
      aria-label="Compatibility preflight"
    >
      <h3>
        {report.errors.length
          ? 'Source can be saved · verification needs attention'
          : 'Static checks passed · independent verification still required'}
      </h3>
      <p>
        {report.modules} modules · {report.source_bytes.toLocaleString()} source bytes
      </p>
      {report.dependencies.length ? (
        <details>
          <summary>Dependency pins</summary>
          <ul>
            {report.dependencies.map((d) => (
              <li key={d}>
                <code>{d}</code>
              </li>
            ))}
          </ul>
        </details>
      ) : null}
      {report.errors.length ? (
        <ul>
          {report.errors.slice(0, 5).map((error, i) => (
            <li key={i}>{error}</li>
          ))}
        </ul>
      ) : null}
      {report.errors.length > 5 ? (
        <details>
          <summary>Show {report.errors.length - 5} more compatibility findings</summary>
          <ul>
            {report.errors.slice(5).map((error, i) => (
              <li key={i}>{error}</li>
            ))}
          </ul>
        </details>
      ) : null}
      {report.warnings.map((warning, i) => (
        <p className="small" key={i}>
          {warning}
        </p>
      ))}
    </section>
  );
}
