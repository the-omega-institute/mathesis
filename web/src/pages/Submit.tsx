import { EvidencePicker } from '../components/EvidenceFiles';
import { bundleManuscript, withEvidence, type EvidenceInput } from '../lib/sourceBundle';
import { useState, type FormEvent } from 'react';
import { LeanDepositForm } from '../components/LeanDeposit';
import { useApi } from '../api/context';
import { PROCESSING_CONSENT_VERSION, PROCESSING_CONSENT_TEXT } from '../api/types';
import type { AiUseLevel, Author, SubmissionKind } from '../api/types';
import { useSession } from '../auth/session';
import { Field, InlineError, Loading, SignInPrompt } from '../components/ui';
import {
  AI_DISCLOSURE_STATEMENTS,
  AI_USES,
  AI_USE_DESCRIPTIONS,
  AI_USE_LABELS,
  formatBytes,
} from '../lib/labels';
import { SOURCE_ACCEPT, buildNewPaper, sourceFileProblem } from '../lib/upload';
import { orcidHref, safeHttpUrl } from '../lib/links';
import { LatexText } from '../components/Markdown';
import { Link, useRouter } from '../routing/router';

type AuthorDraft = {
  name: string;
  affiliation: string;
  orcid: string;
  googleScholar: string;
  website: string;
};

function emptyAuthor(): AuthorDraft {
  return { name: '', affiliation: '', orcid: '', googleScholar: '', website: '' };
}

function authorHasContent(author: AuthorDraft): boolean {
  return Object.values(author).some((value) => value.trim() !== '');
}

function suppliedAuthors(
  drafts: AuthorDraft[],
): { ok: true; value: Author[] } | { ok: false; error: string } {
  const value: Author[] = [];
  for (const [index, draft] of drafts.entries()) {
    const number = index + 1;
    const name = draft.name.trim();
    if (!name) return { ok: false, error: `Give a name for author ${number}.` };
    if (draft.orcid.trim() && !orcidHref(draft.orcid.trim())) {
      return { ok: false, error: `The ORCID for author ${number} is not valid.` };
    }
    for (const [label, raw] of [
      ['Google Scholar URL', draft.googleScholar],
      ['personal website', draft.website],
    ] as const) {
      if (raw.trim() && !safeHttpUrl(raw.trim())) {
        return { ok: false, error: `${label} for author ${number} must be an http(s) URL.` };
      }
    }
    value.push({
      name,
      ...(draft.affiliation.trim() ? { affiliation: draft.affiliation.trim() } : {}),
      ...(draft.orcid.trim() ? { orcid: draft.orcid.trim() } : {}),
      ...(draft.googleScholar.trim() ? { google_scholar: draft.googleScholar.trim() } : {}),
      ...(draft.website.trim() ? { website: draft.website.trim() } : {}),
    });
  }
  return { ok: true, value };
}

export function SubmitPage({
  initialIntake,
  targetRecord,
  targetClaim,
}: { initialIntake?: 'lean' | 'review'; targetRecord?: string; targetClaim?: string } = {}) {
  const [intake, setIntake] = useState<'research' | 'lean' | 'review'>(initialIntake ?? 'research');
  const { session, person } = useSession();
  return (
    <div className="page submission-page">
      <header className="page-head">
        <div>
          <h1>
            {intake === 'lean'
              ? 'Contribute a formalization'
              : intake === 'review'
                ? 'Request a private review'
                : 'Submit your work'}
          </h1>
          <p className="lede">
            {intake === 'review'
              ? 'Submit a manuscript for private research feedback. We check arguments, examine prior work, and identify specific findings.'
              : intake === 'lean'
                ? 'Contribute a Lean proof of an existing conjecture or theorem. Verified contributions credit both the original authors and the formalizers.'
                : 'Submit a paper, short note or conjecture for AI review and a publication decision.'}
          </p>
          <p className="small">
            <Link to={{ kind: 'policy' }}>How work is reviewed</Link>
          </p>
        </div>
      </header>
      {session.status === 'loading' ? <Loading /> : null}
      {session.status !== 'loading' && person === null ? (
        <SignInPrompt what="submit your work" />
      ) : null}
      <div className="button-row" role="group" aria-label="Submission type">
        <button
          className={`button ${intake === 'review' ? '' : 'button-quiet'}`}
          aria-pressed={intake === 'review'}
          onClick={() => setIntake('review')}
        >
          Private review
        </button>
        <button
          className={`button ${intake === 'research' ? '' : 'button-quiet'}`}
          aria-pressed={intake === 'research'}
          onClick={() => setIntake('research')}
        >
          Publication
        </button>
        <button
          className={`button ${intake === 'lean' ? '' : 'button-quiet'}`}
          aria-pressed={intake === 'lean'}
          onClick={() => setIntake('lean')}
        >
          Community formalization
        </button>
      </div>
      {person ? (
        intake !== 'lean' ? (
          <SubmitForm key={intake} reviewOnly={intake === 'review'} />
        ) : (
          <LeanDepositForm targetRecord={targetRecord} targetClaim={targetClaim} />
        )
      ) : null}
    </div>
  );
}

function AuthorFields({
  index,
  author,
  canRemove,
  onChange,
  onRemove,
}: {
  index: number;
  author: AuthorDraft;
  canRemove: boolean;
  onChange: (author: AuthorDraft) => void;
  onRemove: () => void;
}) {
  const patch = (key: keyof AuthorDraft, value: string) => onChange({ ...author, [key]: value });
  return (
    <div className="author-card">
      <h3>Author {index + 1}</h3>
      <div className="form-row">
        <Field label="Name" htmlFor={`author-${index}-name`}>
          <input
            id={`author-${index}-name`}
            value={author.name}
            onChange={(e) => patch('name', e.target.value)}
            autoComplete="name"
          />
        </Field>
        <Field label="Affiliation" htmlFor={`author-${index}-affiliation`}>
          <input
            id={`author-${index}-affiliation`}
            value={author.affiliation}
            onChange={(e) => patch('affiliation', e.target.value)}
            placeholder="University or institute"
            autoComplete="organization"
          />
        </Field>
      </div>
      <details className="author-identifiers">
        <summary>ORCID and academic profiles (optional)</summary>
        <div className="form-row">
          <Field label="ORCID" htmlFor={`author-${index}-orcid`} hint="e.g. 0000-0002-1825-0097">
            <input
              id={`author-${index}-orcid`}
              value={author.orcid}
              onChange={(e) => patch('orcid', e.target.value)}
              placeholder="0000-0002-1825-0097"
              inputMode="text"
            />
          </Field>
          <Field label="Google Scholar" htmlFor={`author-${index}-scholar`}>
            <input
              id={`author-${index}-scholar`}
              type="url"
              value={author.googleScholar}
              onChange={(e) => patch('googleScholar', e.target.value)}
              placeholder="https://scholar.google.com/citations?..."
            />
          </Field>
        </div>
        <Field label="Personal website" htmlFor={`author-${index}-website`}>
          <input
            id={`author-${index}-website`}
            type="url"
            value={author.website}
            onChange={(e) => patch('website', e.target.value)}
            placeholder="https://example.edu/~name"
          />
        </Field>
      </details>
      {canRemove ? (
        <button type="button" className="button button-quiet button-small" onClick={onRemove}>
          Remove author
        </button>
      ) : null}
    </div>
  );
}

export function SubmitForm({ reviewOnly = false }: { reviewOnly?: boolean } = {}) {
  const api = useApi();
  const { person } = useSession();
  const { navigate } = useRouter();
  const [step, setStep] = useState(0);
  const [reviewScope, setReviewScope] = useState('');
  const [kind, setKind] = useState<SubmissionKind>('paper');
  const [mode, setMode] = useState<'upload' | 'type'>('upload');
  const [consent, setConsent] = useState(false);
  const [makePublic, setMakePublic] = useState(true);
  const [title, setTitle] = useState('');
  const [abstract, setAbstract] = useState('');
  const [statement, setStatement] = useState('');
  const [background, setBackground] = useState('');
  const [origin, setOrigin] = useState('');
  const [authors, setAuthors] = useState<AuthorDraft[]>([emptyAuthor()]);
  const [files, setFiles] = useState<File[]>([]);
  const [evidence, setEvidence] = useState<EvidenceInput[]>([]);
  const [aiLevel, setAiLevel] = useState<AiUseLevel>('none');
  const [msc, setMsc] = useState('');
  const [doi, setDoi] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const typed = kind === 'conjecture' && mode === 'type';
  const supplied = authors.filter(authorHasContent);
  const tabs = ['Manuscript', 'Authors', 'Files', 'Review & submit'];
  const changeStep = (next: number) => {
    setStep(next);
    setError(null);
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (step !== 3) {
      changeStep(step + 1);
      return;
    }
    setError(null);
    const fail = (message: string, at: number) => {
      setStep(at);
      setError(message);
    };
    if (!title.trim()) return fail('Enter the manuscript title.', 0);
    if (typed && !statement.trim()) return fail('Enter the conjecture statement.', 0);
    if (!typed && !files.length) return fail('Choose a PDF, LaTeX source, or source archive.', 2);
    if (!supplied.length) return fail('Add the authors in the order shown on the manuscript.', 1);
    const parsedAuthors = suppliedAuthors(supplied);
    if (!parsedAuthors.ok) return fail(parsedAuthors.error, 1);
    const metadata = buildNewPaper({
      aiLevel,
      aiStatement: AI_DISCLOSURE_STATEMENTS[aiLevel],
      msc,
      doi,
      openToContributors: false,
    });
    if (!metadata.ok) return fail(metadata.error, 0);
    if (!consent) return fail('Agree to third-party AI review before uploading.', 3);
    Object.assign(metadata.value, {
      title: title.trim(),
      abstract_text: abstract.trim() || undefined,
      authors: parsedAuthors.value,
      processing_consent: PROCESSING_CONSENT_VERSION,
      kind,
      purpose: reviewOnly ? 'review_only' : 'publication',
      review_scope: reviewScope,
      make_public_after_acceptance: !reviewOnly && makePublic,
      ...(typed
        ? { typed_conjecture: { title: title.trim(), statement, background, origin } }
        : {}),
    });
    setBusy(true);
    try {
      const source = typed
        ? undefined
        : await withEvidence(await bundleManuscript(files), evidence);
      const submission = await api.createSubmission(metadata.value, source);
      navigate({ kind: 'submission', id: submission.id });
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <form
      className="form submission-form submission-wizard"
      onSubmit={(e) => void submit(e)}
      aria-label="Submit work"
      noValidate
    >
      <div className="submission-tabs" role="tablist" aria-label="Submission steps">
        {tabs.map((label, index) => (
          <button
            key={label}
            type="button"
            role="tab"
            id={`submit-tab-${index}`}
            aria-controls={`submit-panel-${index}`}
            aria-selected={step === index}
            tabIndex={step === index ? 0 : -1}
            disabled={busy}
            onKeyDown={(event) => {
              const next =
                event.key === 'ArrowRight'
                  ? (index + 1) % tabs.length
                  : event.key === 'ArrowLeft'
                    ? (index + tabs.length - 1) % tabs.length
                    : event.key === 'Home'
                      ? 0
                      : event.key === 'End'
                        ? tabs.length - 1
                        : null;
              if (next !== null) {
                event.preventDefault();
                changeStep(next);
                document.getElementById(`submit-tab-${next}`)?.focus();
              }
            }}
            onClick={() => changeStep(index)}
          >
            <span className="step-number" aria-hidden="true">
              {index + 1}
            </span>
            {label}
          </button>
        ))}
      </div>
      <div
        className="submission-panel"
        role="tabpanel"
        id={`submit-panel-${step}`}
        aria-labelledby={`submit-tab-${step}`}
      >
        <p className="eyebrow">
          {reviewOnly ? 'Private review' : 'Research submission'} · Step {step + 1} of 4
        </p>
        {step === 0 ? (
          <>
            <h2>Manuscript details</h2>
            <p className="hint">
              Use the title and abstract from your manuscript. Mathematical notation is welcome.
            </p>
            <Field label="Type of work" htmlFor="submission-kind">
              <select
                id="submission-kind"
                value={kind}
                onChange={(e) => setKind(e.target.value as SubmissionKind)}
              >
                <option value="paper">Paper</option>
                <option value="note">Short note</option>
                <option value="conjecture">Conjecture</option>
              </select>
            </Field>
            <Field label="Title" htmlFor="manuscript-title">
              <input
                id="manuscript-title"
                required
                maxLength={1000}
                value={title}
                onChange={(e) => setTitle(e.target.value)}
              />
            </Field>
            <Field label="Abstract" htmlFor="manuscript-abstract">
              <textarea
                id="manuscript-abstract"
                rows={5}
                maxLength={20000}
                value={abstract}
                onChange={(e) => setAbstract(e.target.value)}
              />
            </Field>
            {kind === 'conjecture' ? (
              <>
                <Field label="Manuscript format" htmlFor="conjecture-mode">
                  <select
                    id="conjecture-mode"
                    value={mode}
                    onChange={(e) => setMode(e.target.value as 'upload' | 'type')}
                  >
                    <option value="upload">Upload a manuscript</option>
                    <option value="type">Enter the conjecture here</option>
                  </select>
                </Field>
                {typed ? (
                  <>
                    <Field label="Conjecture statement" htmlFor="conjecture-statement">
                      <textarea
                        id="conjecture-statement"
                        rows={5}
                        required
                        maxLength={20000}
                        value={statement}
                        onChange={(e) => setStatement(e.target.value)}
                      />
                    </Field>
                    {statement ? (
                      <div aria-label="Statement preview">
                        <LatexText source={statement} />
                      </div>
                    ) : null}
                    <Field label="Background (optional)" htmlFor="conjecture-background">
                      <textarea
                        id="conjecture-background"
                        rows={3}
                        maxLength={50000}
                        value={background}
                        onChange={(e) => setBackground(e.target.value)}
                      />
                    </Field>
                    <Field label="Original source (optional)" htmlFor="conjecture-origin">
                      <input
                        id="conjecture-origin"
                        value={origin}
                        maxLength={2000}
                        onChange={(e) => setOrigin(e.target.value)}
                      />
                    </Field>
                  </>
                ) : null}
              </>
            ) : null}
            <div className="form-row">
              <Field label="MSC 2020 codes (optional)" htmlFor="msc" hint="e.g. 11B83, 05D10">
                <input id="msc" value={msc} onChange={(e) => setMsc(e.target.value)} />
              </Field>
              <Field label="DOI (optional)" htmlFor="doi">
                <input
                  id="doi"
                  value={doi}
                  onChange={(e) => setDoi(e.target.value)}
                  placeholder="10.48550/arXiv.…"
                />
              </Field>
            </div>
          </>
        ) : null}
        {step === 1 ? (
          <>
            <h2>Authors and affiliations</h2>
            <p className="hint">
              List all authors in manuscript order. The submitting account is {person?.display_name}
              .
            </p>
            {authors.map((author, index) => (
              <AuthorFields
                key={index}
                index={index}
                author={author}
                canRemove={authors.length > 1}
                onChange={(next) =>
                  setAuthors((current) => current.map((a, i) => (i === index ? next : a)))
                }
                onRemove={() => setAuthors((current) => current.filter((_, i) => i !== index))}
              />
            ))}
            <button
              type="button"
              className="button button-quiet"
              disabled={authors.length >= 50}
              onClick={() => setAuthors((current) => [...current, emptyAuthor()])}
            >
              Add another author
            </button>
          </>
        ) : null}
        {step === 2 ? (
          <>
            <h2>Manuscript files</h2>
            {typed ? (
              <p>Your conjecture will be typeset from the statement entered in Manuscript.</p>
            ) : (
              <>
                <div className="manuscript-upload">
                  <h3>PDF or LaTeX manuscript</h3>
                  <p>
                    Choose a PDF, a source ZIP / tar.gz, or multiple TeX, bibliography and figure
                    files. For a project with subfolders, upload the complete archive.
                  </p>
                  <Field
                    label="Choose manuscript files"
                    htmlFor="source-file"
                    hint="Up to 30 MB. PDF manuscripts must contain selectable text; scanned pages need a text layer."
                  >
                    <input
                      id="source-file"
                      type="file"
                      accept={SOURCE_ACCEPT}
                      multiple
                      onChange={(e) => setFiles(Array.from(e.target.files ?? []))}
                    />
                  </Field>
                  {files.length ? (
                    <ul className="upload-files" aria-label="Selected manuscript files">
                      {files.map((file, i) => (
                        <li key={i}>
                          {file.name}
                          <span>{formatBytes(file.size)}</span>
                        </li>
                      ))}
                    </ul>
                  ) : null}
                  {files.length === 1 && sourceFileProblem(files[0]!) ? (
                    <p className="form-error">{sourceFileProblem(files[0]!)}</p>
                  ) : null}
                </div>
                <details className="history">
                  <summary>Supplementary files (optional)</summary>
                  <EvidencePicker value={evidence} onChange={setEvidence} />
                </details>
              </>
            )}
          </>
        ) : null}
        {step === 3 ? (
          <>
            <h2>Review your submission</h2>
            <dl className="submission-summary">
              <dt>Title</dt>
              <dd>{title || 'Not entered'}</dd>
              <dt>Authors</dt>
              <dd>{supplied.map((a) => a.name).join(', ') || 'Not entered'}</dd>
              <dt>Manuscript</dt>
              <dd>
                {typed
                  ? 'Typed conjecture'
                  : files.map((f) => f.name).join(', ') || 'No files selected'}
              </dd>
              <dt>Purpose</dt>
              <dd>{reviewOnly ? 'Private research feedback' : 'AI review for publication'}</dd>
            </dl>
            <Field
              label="AI use in this work"
              htmlFor="ai-level"
              hint={AI_USE_DESCRIPTIONS[aiLevel]}
            >
              <select
                id="ai-level"
                value={aiLevel}
                onChange={(e) => setAiLevel(e.target.value as AiUseLevel)}
              >
                {AI_USES.map((level) => (
                  <option key={level} value={level}>
                    {AI_USE_LABELS[level]}
                  </option>
                ))}
              </select>
            </Field>
            <Field
              label="Cover note (optional)"
              htmlFor="review-scope"
              hint="Mention a particular result or question you would like the review to address."
            >
              <textarea
                id="review-scope"
                rows={3}
                maxLength={5000}
                value={reviewScope}
                onChange={(e) => setReviewScope(e.target.value)}
              />
            </Field>
            {!reviewOnly ? (
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={makePublic}
                  onChange={(e) => setMakePublic(e.target.checked)}
                />
                <span>Publish the manuscript if accepted</span>
              </label>
            ) : null}
            <p className="hint">
              {reviewOnly
                ? 'Your manuscript and report remain private. This submission does not request publication.'
                : 'The publication result is accepted or not accepted. Accepted work receives a report on its contributions and possible extensions; a refusal explains the reasons.'}
            </p>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={consent}
                onChange={(e) => setConsent(e.target.checked)}
                required
              />
              <span>{PROCESSING_CONSENT_TEXT}</span>
            </label>
          </>
        ) : null}
        <InlineError error={error} />
        <div className="submission-navigation">
          {step > 0 ? (
            <button
              type="button"
              className="button button-quiet"
              disabled={busy}
              onClick={() => changeStep(step - 1)}
            >
              Back
            </button>
          ) : (
            <span />
          )}
          {step < 3 ? (
            <button type="button" className="button" onClick={() => changeStep(step + 1)}>
              Continue
            </button>
          ) : (
            <button type="submit" className="button" disabled={busy}>
              {busy ? 'Uploading…' : 'Upload manuscript'}
            </button>
          )}
        </div>
        {step === 3 ? (
          <p className="hint">You can check the manuscript PDF before starting the review.</p>
        ) : null}
      </div>
    </form>
  );
}
