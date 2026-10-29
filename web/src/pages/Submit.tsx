import { useState, type FormEvent } from 'react';
import { LeanDepositForm } from '../components/LeanDeposit';
import { useApi } from '../api/context';
import { PROCESSING_REQUEST_VERSION } from '../api/types';
import type { AiUseLevel, Submission } from '../api/types';
import { useSession } from '../auth/session';
import { Field, InlineError, Loading, SignInPrompt } from '../components/ui';
import {
  AI_DISCLOSURE_STATEMENTS,
  AI_USES,
  AI_USE_DESCRIPTIONS,
  AI_USE_LABELS,
} from '../lib/labels';
import { buildNewPaper } from '../lib/upload';
import {
  AuthorFields,
  emptyAuthor,
  authorHasContent,
  suppliedAuthors,
  type AuthorDraft,
} from '../components/AuthorFields';
import { PublicationChoice } from '../components/PublicationChoice';
import { UploadSubmission } from '../components/UploadSubmission';
import { LatexText } from '../components/Markdown';
import { useRouter } from '../routing/router';

export function SubmitPage({
  initialIntake,
  targetRecord,
  targetClaim,
  initialSubmission,
  onSubmitted,
}: {
  initialSubmission?: Submission;
  onSubmitted?: (s: Submission) => void;
  initialIntake?: 'lean' | 'review';
  targetRecord?: string;
  targetClaim?: string;
} = {}) {
  const [typedEntry, setTypedEntry] = useState(false);
  const [intake, setIntake] = useState<'research' | 'lean'>(
    initialIntake === 'lean' ? 'lean' : 'research',
  );
  const initiallyPrivate = initialSubmission
    ? initialSubmission.purpose === 'review_only'
    : initialIntake === 'review';
  const { session, person } = useSession();
  return (
    <div className="page submission-page">
      <header className="page-head submission-heading">
        <div>
          <h1>{intake === 'lean' ? 'Contribute a formalization' : 'New submission'}</h1>
          {intake === 'lean' ? (
            <p>
              Submit a Lean proof of an existing conjecture or theorem. Verified contributions
              credit the original authors and the formalizers.
            </p>
          ) : null}
        </div>
        {!initialSubmission ? (
          <button
            type="button"
            className="text-button submission-kind-link"
            onClick={() => setIntake(intake === 'lean' ? 'research' : 'lean')}
          >
            {intake === 'lean' ? 'Submit a manuscript' : 'Community formalization'}
          </button>
        ) : null}
      </header>
      {session.status === 'loading' ? <Loading /> : null}
      {session.status !== 'loading' && person === null ? (
        <SignInPrompt what="submit your work" />
      ) : null}
      {person ? (
        intake !== 'lean' ? (
          typedEntry ? (
            <SubmitForm key={intake} initiallyPrivate={initiallyPrivate} />
          ) : (
            <UploadSubmission
              key={intake}
              initiallyPrivate={initiallyPrivate}
              initialSubmission={initialSubmission}
              onSubmitted={onSubmitted}
            />
          )
        ) : (
          <LeanDepositForm targetRecord={targetRecord} targetClaim={targetClaim} />
        )
      ) : null}
      {person && intake !== 'lean' && !initialSubmission ? (
        <p className="small intake-alternative">
          <button type="button" className="text-button" onClick={() => setTypedEntry(!typedEntry)}>
            {typedEntry
              ? 'Upload a manuscript instead'
              : 'No manuscript file? Write a conjecture here'}
          </button>
        </p>
      ) : null}
    </div>
  );
}

export function SubmitForm({ initiallyPrivate = false }: { initiallyPrivate?: boolean } = {}) {
  const api = useApi();
  const { person } = useSession();
  const { navigate } = useRouter();
  const [step, setStep] = useState(0);
  const [reviewScope, setReviewScope] = useState('');
  const [makePublic, setMakePublic] = useState(!initiallyPrivate);
  const [title, setTitle] = useState('');
  const [abstract, setAbstract] = useState('');
  const [statement, setStatement] = useState('');
  const [background, setBackground] = useState('');
  const [origin, setOrigin] = useState('');
  const [authors, setAuthors] = useState<AuthorDraft[]>([emptyAuthor()]);
  const [aiLevel, setAiLevel] = useState<AiUseLevel>('none');
  const [msc, setMsc] = useState('');
  const [doi, setDoi] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const supplied = authors.filter(authorHasContent);
  const tabs = ['Conjecture', 'Authors', 'Review & submit'];
  const changeStep = (next: number) => {
    setStep(next);
    setError(null);
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (step !== 2) {
      changeStep(step + 1);
      return;
    }
    setError(null);
    const fail = (message: string, at: number) => {
      setStep(at);
      setError(message);
    };
    if (!title.trim()) return fail('Enter the manuscript title.', 0);
    if (!statement.trim()) return fail('Enter the conjecture statement.', 0);
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
    Object.assign(metadata.value, {
      title: title.trim(),
      abstract_text: abstract.trim() || undefined,
      authors: parsedAuthors.value,
      processing_consent: PROCESSING_REQUEST_VERSION,
      kind: 'conjecture',
      purpose: makePublic ? 'publication' : 'review_only',
      review_scope: reviewScope,
      make_public_after_acceptance: makePublic,
      typed_conjecture: { title: title.trim(), statement, background, origin },
    });
    setBusy(true);
    try {
      const submission = await api.createSubmission(metadata.value);
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
        <p className="eyebrow">Research submission · Step {step + 1} of 3</p>
        {step === 0 ? (
          <>
            <h2>State your conjecture</h2>
            <p className="hint">
              Give the conjecture a title and a precise statement. Mathematical notation is welcome.
            </p>
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
            <details className="plain-details">
              <summary>AI use in the manuscript (optional)</summary>
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
            </details>
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
            <h2>Review your submission</h2>
            <dl className="submission-summary">
              <dt>Title</dt>
              <dd>{title || 'Not entered'}</dd>
              <dt>Authors</dt>
              <dd>{supplied.map((a) => a.name).join(', ') || 'Not entered'}</dd>
              <dt>Manuscript</dt>
              <dd>Typed conjecture</dd>
              <dt>Purpose</dt>
              <dd>{makePublic ? 'AI review for publication' : 'Private review feedback'}</dd>
            </dl>
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
            <PublicationChoice checked={makePublic} onChange={setMakePublic} />
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
          {step < 2 ? (
            <button type="button" className="button" onClick={() => changeStep(step + 1)}>
              Continue
            </button>
          ) : (
            <button type="submit" className="button" disabled={busy}>
              {busy ? 'Preparing…' : 'Prepare conjecture PDF'}
            </button>
          )}
        </div>
        {step === 2 ? (
          <p className="hint">You can check the manuscript PDF before starting the review.</p>
        ) : null}
      </div>
    </form>
  );
}
