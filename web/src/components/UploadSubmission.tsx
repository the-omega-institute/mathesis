import { useEffect, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { pdfHref } from '../api/client';
import { PROCESSING_REQUEST_VERSION } from '../api/types';
import type { AiUseLevel, Submission } from '../api/types';
import { useRouter } from '../routing/router';
import { bundleManuscript, withEvidence, type EvidenceInput } from '../lib/sourceBundle';
import { SOURCE_ACCEPT, buildNewPaper } from '../lib/upload';
import { buildConfirmations, initialRows, updateRow } from '../lib/confirm';
import {
  AI_DISCLOSURE_STATEMENTS,
  AI_USES,
  AI_USE_LABELS,
  AI_USE_DESCRIPTIONS,
  formatBytes,
} from '../lib/labels';
import { AuthorFields, emptyAuthor, suppliedAuthors, type AuthorDraft } from './AuthorFields';
import { PublicationChoice } from './PublicationChoice';
import { EvidencePicker } from './EvidenceFiles';
import { Field, InlineError } from './ui';
import { LatexText } from './Markdown';

const steps = ['Upload', 'Check PDF', 'Details', 'Authors', 'Submit'];
const authorDrafts = (s?: Submission): AuthorDraft[] =>
  s?.authors.length
    ? s.authors.map((a) => ({
        name: a.name,
        affiliation: a.affiliation ?? '',
        orcid: a.orcid ?? '',
        googleScholar: a.google_scholar ?? '',
        website: a.website ?? '',
      }))
    : [emptyAuthor()];

export function UploadSubmission({
  initiallyPrivate,
  initialSubmission,
  onSubmitted,
}: {
  initiallyPrivate: boolean;
  initialSubmission?: Submission;
  onSubmitted?: (s: Submission) => void;
}) {
  const api = useApi();
  const { navigate } = useRouter();
  const [draft, setDraft] = useState(initialSubmission);
  const [step, setStep] = useState(initialSubmission ? 1 : 0);
  const kind = draft?.kind ?? 'paper';
  const [files, setFiles] = useState<File[]>([]);
  const [evidence, setEvidence] = useState<EvidenceInput[]>([]);
  const [pdfChecked, setPdfChecked] = useState(false);
  const [title, setTitle] = useState(initialSubmission?.title ?? '');
  const [abstract, setAbstract] = useState(initialSubmission?.abstract_text ?? '');
  const [authors, setAuthors] = useState(() => authorDrafts(initialSubmission));
  const [msc, setMsc] = useState(initialSubmission?.msc.join(', ') ?? '');
  const [doi, setDoi] = useState(initialSubmission?.doi ?? '');
  const [aiLevel, setAiLevel] = useState<AiUseLevel>('none');
  const [reviewScope, setReviewScope] = useState('');
  const [makePublic, setMakePublic] = useState(!initiallyPrivate);
  const [rows, setRows] = useState(() => initialRows(initialSubmission?.extracted ?? []));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [pollError, setPollError] = useState<unknown>(null);
  const version = draft?.versions.at(-1);
  const ready = !!version?.pdf && !version.compile_error;
  const preparing = !!draft && !version?.pdf && !version?.compile_error;

  const draftId = draft?.id;
  useEffect(() => {
    if (!preparing || !draftId) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        const current = await api.getSubmission(draftId, { signal: controller.signal });
        if (!controller.signal.aborted) {
          setDraft(current);
          setPollError(null);
        }
      } catch (e) {
        if (!controller.signal.aborted) setPollError(e);
      }
      if (!controller.signal.aborted) timer = setTimeout(() => void refresh(), 2500);
    };
    void refresh();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [api, draftId, preparing]);

  const changeStep = (next: number) => {
    setError(null);
    setStep(next);
  };
  const upload = async () => {
    if (!files.length) {
      setError('Choose a PDF, LaTeX file or source archive.');
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const source = await withEvidence(await bundleManuscript(files), evidence);
      const s = draft
        ? await api.uploadVersion(
            draft.id,
            source,
            'Updated during submission preparation',
            PROCESSING_REQUEST_VERSION,
          )
        : await api.prepareSubmission(
            {
              kind,
              purpose: makePublic ? 'publication' : 'review_only',
              processing_consent: PROCESSING_REQUEST_VERSION,
            },
            source,
          );
      setDraft(s);
      setTitle(s.title);
      setAbstract(s.abstract_text);
      setAuthors(authorDrafts(s));
      setRows(initialRows(s.extracted));
      setPdfChecked(false);
      setStep(1);
      if (!draft) navigate({ kind: 'submission', id: s.id });
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const validateDetails = () => {
    if (!title.trim()) {
      setError('Enter the manuscript title.');
      return false;
    }
    const metadata = buildNewPaper({
      aiLevel,
      aiStatement: AI_DISCLOSURE_STATEMENTS[aiLevel],
      msc,
      doi,
      openToContributors: false,
    });
    if (!metadata.ok) {
      setError(metadata.error);
      return false;
    }
    return true;
  };
  const next = async () => {
    setError(null);
    if (step === 0) {
      if (draft && !files.length) changeStep(1);
      else await upload();
      return;
    }
    if (step === 1) {
      if (!ready) return;
      setPdfChecked(true);
    }
    if (step === 2 && !validateDetails()) return;
    if (step === 3) {
      const parsed = suppliedAuthors(authors);
      if (!parsed.ok) {
        setError(parsed.error);
        return;
      }
    }
    changeStep(step + 1);
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (step < 4) {
      await next();
      return;
    }
    setError(null);
    if (!draft || !version || !ready || !pdfChecked) {
      setStep(1);
      setError('Check the prepared PDF first.');
      return;
    }
    if (!validateDetails()) {
      setStep(2);
      return;
    }
    const parsed = suppliedAuthors(authors);
    if (!parsed.ok) {
      setStep(3);
      setError(parsed.error);
      return;
    }
    const claims = buildConfirmations(rows, kind === 'conjecture');
    if (!claims.ok) {
      setError(claims.error);
      return;
    }
    const built = buildNewPaper({
      aiLevel,
      aiStatement: AI_DISCLOSURE_STATEMENTS[aiLevel],
      msc,
      doi,
      openToContributors: false,
    });
    if (!built.ok) return;
    setBusy(true);
    try {
      const s = await api.submitPrepared(draft.id, {
        version: version.number,
        source_sha256: version.archive.sha256,
        claims_revision: draft.claims_revision,
        metadata: {
          ...built.value,
          kind,
          purpose: makePublic ? 'publication' : 'review_only',
          title: title.trim(),
          abstract_text: abstract.trim(),
          authors: parsed.value,
          processing_consent: PROCESSING_REQUEST_VERSION,
          make_public_after_acceptance: makePublic,
          review_scope: reviewScope,
        },
        claims: claims.value,
      });
      if (onSubmitted) onSubmitted(s);
      else navigate({ kind: 'submission', id: s.id });
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const accessibleStep = (index: number) =>
    index === 0 || (!!draft && (index === 1 || (ready && pdfChecked)));

  return (
    <form
      className="form submission-form submission-wizard"
      aria-label="Submit work"
      noValidate
      onSubmit={(e) => void submit(e)}
    >
      <div className="submission-tabs" role="tablist" aria-label="Submission steps">
        {steps.map((label, index) => (
          <button
            type="button"
            key={label}
            role="tab"
            id={`upload-tab-${index}`}
            aria-controls={`upload-panel-${index}`}
            aria-selected={step === index}
            tabIndex={step === index ? 0 : -1}
            disabled={busy || !accessibleStep(index)}
            onClick={() => changeStep(index)}
            onKeyDown={(e) => {
              if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) return;
              e.preventDefault();
              const available = steps.map((_, i) => i).filter(accessibleStep);
              const at = available.indexOf(index);
              const nextIndex =
                e.key === 'Home'
                  ? available[0]!
                  : e.key === 'End'
                    ? available.at(-1)!
                    : available[
                        (at + (e.key === 'ArrowRight' ? 1 : available.length - 1)) %
                          available.length
                      ]!;
              changeStep(nextIndex);
              document.getElementById(`upload-tab-${nextIndex}`)?.focus();
            }}
          >
            <span className="step-number" aria-hidden="true">
              {index + 1}.
            </span>
            {label}
          </button>
        ))}
      </div>
      <div
        className={`submission-panel${step === 1 ? ' submission-preview' : ''}`}
        role="tabpanel"
        id={`upload-panel-${step}`}
        aria-labelledby={`upload-tab-${step}`}
      >
        {step === 0 ? (
          <>
            <h2>Upload your manuscript</h2>
            <p className="submission-instruction">
              Start with your paper. We will prepare a PDF and fill in the details we can read from
              it.
            </p>
            <Field
              label={draft ? 'Replace manuscript files' : 'Choose manuscript files'}
              htmlFor="source-file"
              hint="PDF, .tex, .zip or .tar.gz · up to 30 MB. You may select several TeX, bibliography and figure files together."
            >
              <input
                id="source-file"
                type="file"
                accept={SOURCE_ACCEPT}
                multiple
                onChange={(e) => {
                  setFiles(Array.from(e.target.files ?? []));
                  setError(null);
                }}
              />
            </Field>
            <p className="hint">
              For projects with subfolders, upload a ZIP or tar.gz containing the complete source.
              PDF files need selectable text.
            </p>
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
            {draft && !files.length ? (
              <p className="small">Saved manuscript: {version?.filename}</p>
            ) : null}
            <details className="plain-details">
              <summary>Supplementary material (optional)</summary>
              <EvidencePicker value={evidence} onChange={setEvidence} />
            </details>
          </>
        ) : null}
        {step === 1 ? (
          <>
            <div className="pdf-heading">
              <div>
                <h2>Check your manuscript</h2>
                <p>{version?.filename}</p>
              </div>
              {ready && draft ? (
                <a href={pdfHref(draft.id, version?.number)} target="_blank" rel="noreferrer">
                  Open PDF in a new tab
                </a>
              ) : null}
            </div>
            {preparing ? (
              <p role="status" className="preparation-status">
                Preparing your PDF…
              </p>
            ) : null}
            <InlineError error={pollError} />
            {version?.compile_error ? (
              <div role="alert">
                <p className="form-error">
                  The manuscript could not be compiled. Correct the source files and upload them
                  again.
                </p>
                <details className="plain-details">
                  <summary>Compilation details</summary>
                  <pre>{version.compile_error}</pre>
                </details>
                <button type="button" className="button button-quiet" onClick={() => changeStep(0)}>
                  Replace files
                </button>
              </div>
            ) : null}
            {ready && draft ? (
              <>
                <iframe
                  className="manuscript-pdf"
                  title="Manuscript PDF preview"
                  src={pdfHref(draft.id, version?.number)}
                />
                {version?.parse_warnings.length ? (
                  <div className="extraction-notes">
                    <p>
                      Please check the mathematical notation and statement boundaries in the PDF.
                    </p>
                    <details className="plain-details">
                      <summary>Reading notes</summary>
                      <ul>
                        {version.parse_warnings.map((w, i) => (
                          <li key={i}>{w}</li>
                        ))}
                      </ul>
                    </details>
                  </div>
                ) : null}
              </>
            ) : null}
          </>
        ) : null}
        {step === 2 ? (
          <>
            <h2>Manuscript details</h2>
            <p className="submission-instruction">
              Check the title and abstract read from your manuscript. You can edit either field.
            </p>
            <Field label="Title" htmlFor="manuscript-title">
              <input
                id="manuscript-title"
                value={title}
                maxLength={1000}
                onChange={(e) => setTitle(e.target.value)}
              />
            </Field>
            <Field
              label="Abstract"
              htmlFor="manuscript-abstract"
              hint="LaTeX mathematical notation is supported."
            >
              <textarea
                id="manuscript-abstract"
                value={abstract}
                rows={7}
                maxLength={20000}
                onChange={(e) => setAbstract(e.target.value)}
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
            <details className="plain-details">
              <summary>Classification and DOI (optional)</summary>
              <div className="form-row">
                <Field label="MSC 2020 codes (optional)" htmlFor="msc">
                  <input
                    id="msc"
                    value={msc}
                    onChange={(e) => setMsc(e.target.value)}
                    placeholder="11B83, 05D10"
                  />
                </Field>
                <Field label="DOI (optional)" htmlFor="doi">
                  <input id="doi" value={doi} onChange={(e) => setDoi(e.target.value)} />
                </Field>
              </div>
            </details>
          </>
        ) : null}
        {step === 3 ? (
          <>
            <h2>Authors and affiliations</h2>
            <p className="submission-instruction">
              Check the names and their order against the manuscript. Add affiliations where
              appropriate.
            </p>
            {authors.map((author, i) => (
              <AuthorFields
                key={i}
                index={i}
                author={author}
                canRemove={authors.length > 1}
                onChange={(a) =>
                  setAuthors((current) => current.map((old, n) => (n === i ? a : old)))
                }
                onRemove={() => setAuthors((current) => current.filter((_, n) => n !== i))}
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
        {step === 4 ? (
          <>
            <h2>Review and submit</h2>
            <div className="manuscript-summary">
              <h3>{title || 'Untitled manuscript'}</h3>
              <p>{authors.map((a) => a.name).join(', ')}</p>
              {abstract ? <LatexText source={abstract} /> : null}
              <p className="small">
                {version?.filename} ·{' '}
                <button type="button" className="text-button" onClick={() => changeStep(1)}>
                  Check PDF
                </button>
              </p>
            </div>
            <Field label="Cover note (optional)" htmlFor="review-scope">
              <textarea
                id="review-scope"
                rows={3}
                value={reviewScope}
                maxLength={5000}
                onChange={(e) => setReviewScope(e.target.value)}
              />
            </Field>
            <details className="plain-details">
              <summary>Identified results ({rows.length})</summary>
              <p className="hint">
                These statements will be reviewed. Adjust a main result or exclude a statement if
                needed.
              </p>
              {rows.map((row) => (
                <div className="intake-claim" key={row.id}>
                  <p>
                    <strong>{draft?.extracted.find((c) => c.id === row.id)?.label}</strong>
                  </p>
                  <LatexText
                    source={draft?.extracted.find((c) => c.id === row.id)?.statement ?? ''}
                  />
                  <div className="button-row">
                    <label className="checkbox">
                      <input
                        type="checkbox"
                        checked={row.role === 'main'}
                        onChange={(e) =>
                          setRows((current) =>
                            updateRow(current, row.id, {
                              role: e.target.checked ? 'main' : 'supporting',
                            }),
                          )
                        }
                      />
                      Main result
                    </label>
                    <label className="checkbox">
                      <input
                        type="checkbox"
                        checked={row.excluded}
                        onChange={(e) =>
                          setRows((current) =>
                            updateRow(current, row.id, { excluded: e.target.checked }),
                          )
                        }
                      />
                      Exclude
                    </label>
                  </div>
                </div>
              ))}
            </details>
            <PublicationChoice checked={makePublic} onChange={setMakePublic} />
          </>
        ) : null}
        <InlineError error={error} />
        <div className="submission-navigation">
          <div>
            {step > 0 ? (
              <button
                type="button"
                className="button button-quiet"
                disabled={busy}
                onClick={() => changeStep(step - 1)}
              >
                Back
              </button>
            ) : null}
          </div>
          <button type="submit" className="button" disabled={busy || (step === 1 && !ready)}>
            {busy
              ? step === 0
                ? 'Uploading…'
                : 'Submitting…'
              : step === 0
                ? draft && !files.length
                  ? 'Return to PDF'
                  : 'Upload and prepare PDF'
                : step === 4
                  ? 'Submit for review'
                  : step === 1
                    ? 'Continue to details'
                    : 'Continue'}
          </button>
        </div>
      </div>
    </form>
  );
}
