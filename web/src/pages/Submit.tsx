import { EvidencePicker } from '../components/EvidenceFiles';
import { withEvidence, type EvidenceInput } from '../lib/sourceBundle';
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
import { SOURCE_ACCEPT, buildNewPaper, pdfCopyProblem, sourceFileProblem } from '../lib/upload';
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
              : 'Share research or contribute a Lean proof. Mathesis provides research review and independent Lean verification.'}
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
          Paper or conjecture
        </button>
        <button
          className={`button ${intake === 'lean' ? '' : 'button-quiet'}`}
          aria-pressed={intake === 'lean'}
          onClick={() => setIntake('lean')}
        >
          Lean formalization
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
  const [reviewScope, setReviewScope] = useState('');
  const [kind, setKind] = useState<SubmissionKind>('paper');
  const [mode, setMode] = useState<'upload' | 'type'>('upload');
  const [consent, setConsent] = useState(false);
  const [makePublic, setMakePublic] = useState(true);
  const [title, setTitle] = useState('');
  const [statement, setStatement] = useState('');
  const [background, setBackground] = useState('');
  const [origin, setOrigin] = useState('');
  const [authorName, setAuthorName] = useState(person?.display_name ?? '');
  const [authors, setAuthors] = useState<AuthorDraft[]>([emptyAuthor()]);
  const typed = kind === 'conjecture' && mode === 'type';
  const { navigate } = useRouter();
  const [file, setFile] = useState<File | null>(null);
  const [pdfCopy, setPdfCopy] = useState<File | null>(null);
  const [evidence, setEvidence] = useState<EvidenceInput[]>([]);
  const [aiLevel, setAiLevel] = useState<AiUseLevel>('none');
  const [msc, setMsc] = useState('');
  const [doi, setDoi] = useState('');
  const [openToContributors, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const fileProblem = file ? sourceFileProblem(file) : null;
  const pdfProblem = pdfCopy ? pdfCopyProblem(pdfCopy) : null;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    if (!typed && file === null) {
      setError('Choose the LaTeX source file.');
      return;
    }
    if (!typed && fileProblem) {
      setError(fileProblem);
      return;
    }
    if (!typed && pdfProblem) {
      setError(pdfProblem);
      return;
    }
    const metadata = buildNewPaper({
      aiLevel,
      aiStatement: AI_DISCLOSURE_STATEMENTS[aiLevel],
      msc,
      doi,
      openToContributors,
    });
    if (!metadata.ok) {
      setError(metadata.error);
      return;
    }
    if (!consent) {
      setError('Agree to third-party AI review before uploading.');
      return;
    }
    metadata.value.processing_consent = PROCESSING_CONSENT_VERSION;
    metadata.value.kind = kind;
    metadata.value.purpose = reviewOnly ? 'review_only' : 'publication';
    metadata.value.review_scope = reviewScope;
    metadata.value.make_public_after_acceptance = !reviewOnly && makePublic;
    if (typed) {
      if (!title.trim() || !statement.trim() || !authorName.trim()) {
        setError('Give a title, statement and author name.');
        return;
      }
      if (Array.from(statement).length > 20_000 || Array.from(background).length > 50_000) {
        setError('Keep the statement within 20,000 characters and background within 50,000.');
        return;
      }
      metadata.value.typed_conjecture = { title, statement, background, origin };
      metadata.value.authors = [{ name: authorName, person: person?.id }];
    } else {
      const supplied = authors.filter((author) => authorHasContent(author));
      if (supplied.length > 0) {
        const parsed = suppliedAuthors(supplied);
        if (!parsed.ok) {
          setError(parsed.error);
          return;
        }
        metadata.value.authors = parsed.value;
      }
    }
    setBusy(true);
    const attachments = pdfCopy
      ? [
          ...evidence,
          {
            file: pdfCopy,
            kind: 'supplement' as const,
            description: 'Author-provided PDF reading copy.',
            claims: [],
          },
        ]
      : evidence;
    (typed ? Promise.resolve(undefined) : withEvidence(file as File, attachments))
      .then((source) => api.createSubmission(metadata.value, source))
      .then(
        (s) => {
          setBusy(false);
          navigate({ kind: 'submission', id: s.id });
        },
        (e: unknown) => {
          setBusy(false);
          setError(e);
        },
      );
  };

  return (
    <form className="form submission-form" onSubmit={submit} aria-label="Submit work" noValidate>
      {reviewOnly ? (
        <div className="notice notice-info">
          <strong>Private research feedback</strong>
          <p>
            Get your arguments checked without applying for publication. Your manuscript and review
            remain available only to you, linked co-authors and review staff.
          </p>
        </div>
      ) : null}
      <section aria-labelledby="upload-primer-h" className="notice notice-info">
        <h2 id="upload-primer-h">What happens after you upload</h2>
        <p>
          Upload → you confirm the statement list → the review runs → the feedback letter arrives on
          this site{reviewOnly ? '.' : ' → accepted work receives a WP record.'}
        </p>
        <p className="small">You can return here to read the letter; delivery is on this site.</p>
      </section>
      <section className="submission-manuscript" aria-labelledby="manuscript-heading">
        <h2 id="manuscript-heading">Manuscript</h2>
        <fieldset className="submission-choices">
          <legend>Type of work (defaults to Paper)</legend>
          {(['paper', 'note', 'conjecture'] as const).map((value) => (
            <label key={value} className="radio">
              <input
                type="radio"
                name="submission-kind"
                checked={kind === value}
                onChange={() => setKind(value)}
              />
              <span>
                {value === 'paper' ? 'Paper' : value === 'note' ? 'Short note' : 'Conjecture'}
              </span>
            </label>
          ))}
        </fieldset>
        {kind === 'conjecture' ? (
          <fieldset className="submission-choices">
            <legend>How would you like to enter it?</legend>
            <label className="radio">
              <input
                type="radio"
                name="conjecture-mode"
                checked={mode === 'upload'}
                onChange={() => setMode('upload')}
              />
              Upload LaTeX
            </label>
            <label className="radio">
              <input
                type="radio"
                name="conjecture-mode"
                checked={mode === 'type'}
                onChange={() => setMode('type')}
              />
              Type it
            </label>
          </fieldset>
        ) : null}
        {typed ? (
          <div key="typed-manuscript">
            <Field
              label="Title (required)"
              htmlFor="conjecture-title"
              hint="We use the title to identify your conjecture."
            >
              <input
                id="conjecture-title"
                required
                value={title}
                maxLength={1000}
                onChange={(e) => setTitle(e.target.value)}
              />
            </Field>
            <Field
              label="Author (required)"
              htmlFor="conjecture-author"
              hint="We use this name to attribute your conjecture."
            >
              <input
                id="conjecture-author"
                required
                value={authorName}
                onChange={(e) => setAuthorName(e.target.value)}
              />
            </Field>
            <Field
              label="Statement (required)"
              htmlFor="conjecture-statement"
              hint="We review this statement to assess the conjecture. TeX math is welcome. At most 20,000 characters."
            >
              <textarea
                id="conjecture-statement"
                required
                rows={6}
                maxLength={20000}
                value={statement}
                onChange={(e) => setStatement(e.target.value)}
              />
            </Field>
            {statement ? (
              <div className="typed-statement-preview" aria-label="Statement preview">
                <LatexText source={statement} />
              </div>
            ) : null}
          </div>
        ) : (
          <div key="source-manuscript">
            <Field
              label="LaTeX source (required)"
              htmlFor="source-file"
              hint="We read the source to identify your statements and check their arguments. A single .tex file, or a .zip or .tar.gz archive with the main file, bibliography and figures. At most 30 MB."
            >
              <input
                id="source-file"
                type="file"
                accept={SOURCE_ACCEPT}
                required
                onChange={(e) => setFile(e.target.files?.[0] ?? null)}
              />
            </Field>
            {file ? (
              <p className="small" aria-live="polite">
                {file.name} · {formatBytes(file.size)}
                {fileProblem ? <span className="form-error"> — {fileProblem}</span> : null}
              </p>
            ) : null}
          </div>
        )}
        <fieldset className="radio-list">
          <legend>AI use in this work (required)</legend>
          <p className="hint">
            We ask so readers understand how the mathematics and text were produced.
          </p>
          <p className="hint">
            Choose the closest description. Add fuller disclosure directly in the manuscript if
            needed.
          </p>
          {AI_USES.map((level) => (
            <label key={level} className="radio">
              <input
                type="radio"
                name="ai-level"
                value={level}
                checked={aiLevel === level}
                onChange={() => setAiLevel(level)}
              />
              <span>
                <strong>{AI_USE_LABELS[level]}</strong> — {AI_USE_DESCRIPTIONS[level]}
              </span>
            </label>
          ))}
        </fieldset>

        <label className="checkbox">
          <input
            type="checkbox"
            checked={consent}
            onChange={(e) => setConsent(e.target.checked)}
            required
          />
          <span>Processing consent (required): {PROCESSING_CONSENT_TEXT}</span>
        </label>
        <p className="hint">
          We need your permission to send the manuscript to the review services that read and check
          it.
        </p>
        <div className="button-row">
          <button type="submit" className="button" disabled={busy}>
            {busy ? 'Uploading…' : 'Upload and read statements'}
          </button>
        </div>
        <InlineError error={error} />
      </section>
      <section className="submission-options" aria-labelledby="review-options-heading">
        <h2 id="review-options-heading">Optional information and preferences</h2>
        <p className="hint">
          Everything in this section is optional. You can leave the defaults and upload.
        </p>
        {typed ? (
          <>
            <Field label="Background (optional)" htmlFor="conjecture-background">
              <textarea
                id="conjecture-background"
                rows={4}
                maxLength={50000}
                value={background}
                onChange={(e) => setBackground(e.target.value)}
              />
            </Field>
            <Field
              label="Origin (optional)"
              htmlFor="conjecture-origin"
              hint="My own, or a work or URL as you know it."
            >
              <input
                id="conjecture-origin"
                maxLength={2000}
                value={origin}
                onChange={(e) => setOrigin(e.target.value)}
              />
            </Field>
          </>
        ) : null}
        {!typed ? (
          <>
            <Field
              label="PDF manuscript or reading copy (optional)"
              htmlFor="pdf-copy"
              hint="Useful when your manuscript comes from Word or PDF. LaTeX is still required for automatic statement extraction, compilation and review. At most 30 MB."
            >
              <input
                id="pdf-copy"
                type="file"
                accept="application/pdf,.pdf"
                onChange={(e) => setPdfCopy(e.target.files?.[0] ?? null)}
              />
            </Field>
            {pdfCopy ? (
              <p className="small" aria-live="polite">
                {pdfCopy.name} · {formatBytes(pdfCopy.size)}
                {pdfProblem ? <span className="form-error"> — {pdfProblem}</span> : null}
              </p>
            ) : null}
            <EvidencePicker value={evidence} onChange={setEvidence} />
            <fieldset className="author-fields">
              <legend>Authors (optional)</legend>
              <p className="hint">
                Leave this blank to read the author list from the manuscript. Add details here when
                the PDF or source does not contain them; these entries replace the source author
                list.
              </p>
              {authors.map((author, index) => (
                <AuthorFields
                  key={index}
                  index={index}
                  author={author}
                  canRemove={authors.length > 1}
                  onChange={(next) =>
                    setAuthors((current) => current.map((item, i) => (i === index ? next : item)))
                  }
                  onRemove={() => setAuthors((current) => current.filter((_, i) => i !== index))}
                />
              ))}
              <button
                type="button"
                className="button button-quiet button-small"
                onClick={() => setAuthors((current) => [...current, emptyAuthor()])}
              >
                Add another author
              </button>
            </fieldset>
          </>
        ) : null}
        <Field
          label="What should the review focus on? (optional)"
          htmlFor="review-scope"
          hint="Name the statements, proof steps or literature questions that matter most. All confirmed statements still receive an audit reading."
        >
          <textarea
            id="review-scope"
            maxLength={5000}
            rows={3}
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
            <span>Make public after acceptance (optional)</span>
          </label>
        ) : null}
        <p className="hint">Reviews and letters are only shown to you.</p>

        <div className="form-row">
          <Field label="MSC 2020 codes (optional)" htmlFor="msc" hint="e.g. 11B83, 05D10">
            <input id="msc" value={msc} onChange={(e) => setMsc(e.target.value)} />
          </Field>
          <Field
            label="DOI (optional)"
            htmlFor="doi"
            hint="If the paper already has one, e.g. 10.48550/arXiv.2609.33421"
          >
            <input id="doi" value={doi} onChange={(e) => setDoi(e.target.value)} />
          </Field>
        </div>

        {!reviewOnly ? (
          <label className="checkbox">
            <input
              type="checkbox"
              checked={openToContributors}
              onChange={(e) => setOpen(e.target.checked)}
            />
            <span>
              Optional: let volunteer contributors help analyse the paper. They will see the
              statements, the title and the abstract while the paper is in review. You can change
              this later.
            </span>
          </label>
        ) : null}
      </section>
    </form>
  );
}
