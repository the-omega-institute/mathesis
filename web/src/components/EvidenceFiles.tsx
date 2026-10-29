import { safeHttpUrl } from '../lib/links';
import type { EvidenceAttachment } from '../api/types';
import type { EvidenceInput } from '../lib/sourceBundle';
import { Field } from './ui';

export function EvidencePicker({
  value,
  onChange,
}: {
  value: EvidenceInput[];
  onChange: (files: EvidenceInput[]) => void;
}) {
  const patch = (index: number, update: Partial<EvidenceInput>) =>
    onChange(value.map((entry, i) => (i === index ? { ...entry, ...update } : entry)));
  return (
    <fieldset className="evidence-picker">
      <legend>Computational evidence & supplements (optional)</legend>
      <p className="hint">
        Attach certificates, computation logs, data, a PDF reading copy or source files. They are
        saved with this manuscript version and included in the review inventory. A supplied
        certificate is not yet independently verified.
      </p>
      <label>
        Attach evidence files (optional)
        <input
          type="file"
          multiple
          onChange={(event) => {
            onChange([
              ...value,
              ...Array.from(event.target.files ?? []).map((file): EvidenceInput => ({
                file,
                kind: 'supplement',
                description: '',
                claims: [],
              })),
            ]);
            event.target.value = '';
          }}
        />
      </label>
      {value.map((entry, index) => (
        <div className="evidence-card" key={`${index}-${entry.file.name}`}>
          <strong>{entry.file.name}</strong>{' '}
          <span className="small muted">{Math.ceil(entry.file.size / 1024)} KB</span>
          <Field label="Evidence type" htmlFor={`evidence-${index}-kind`}>
            <select
              id={`evidence-${index}-kind`}
              value={entry.kind}
              onChange={(e) => patch(index, { kind: e.target.value as EvidenceAttachment['kind'] })}
            >
              <option value="certificate">Certificate</option>
              <option value="computation">Computation / log</option>
              <option value="source">Source code</option>
              <option value="data">Data</option>
              <option value="supplement">Supplement</option>
            </select>
          </Field>
          <Field label="What does this support?" htmlFor={`evidence-${index}-description`}>
            <textarea
              id={`evidence-${index}-description`}
              required
              maxLength={4000}
              value={entry.description}
              onChange={(e) => patch(index, { description: e.target.value })}
            />
          </Field>
          <Field
            label="Statement labels (optional, comma-separated)"
            htmlFor={`evidence-${index}-claims`}
          >
            <input
              id={`evidence-${index}-claims`}
              placeholder="thm:main, C1"
              value={entry.claims.join(', ')}
              onChange={(e) =>
                patch(index, { claims: e.target.value.split(',').map((s) => s.trim()) })
              }
            />
          </Field>
          <Field label="Source URL (optional)" htmlFor={`evidence-${index}-url`}>
            <input
              id={`evidence-${index}-url`}
              type="url"
              value={entry.source_url ?? ''}
              onChange={(e) => patch(index, { source_url: e.target.value || undefined })}
            />
          </Field>
          <Field label="Full Git commit (optional)" htmlFor={`evidence-${index}-revision`}>
            <input
              id={`evidence-${index}-revision`}
              value={entry.source_revision ?? ''}
              onChange={(e) => patch(index, { source_revision: e.target.value || undefined })}
            />
          </Field>
          <button
            type="button"
            className="button button-quiet button-small"
            onClick={() => onChange(value.filter((_, i) => i !== index))}
          >
            Remove {entry.file.name}
          </button>
        </div>
      ))}
    </fieldset>
  );
}
export function EvidenceInventory({
  files,
  submission,
  version,
  technicalDetails = false,
}: {
  files?: EvidenceAttachment[];
  submission: string;
  version: number;
  technicalDetails?: boolean;
}) {
  if (!files?.length) return null;
  return (
    <section className="evidence-inventory" aria-label={`Evidence for version ${version}`}>
      <h3>Supplementary material · version {version}</h3>
      <p className="hint">
        Additional material supplied by the authors with this manuscript version.
      </p>
      <ul className="entry-list">
        {files.map((file) => (
          <li key={file.path}>
            <a
              href={`/api/v1/submissions/${encodeURIComponent(submission)}/evidence?version=${version}&path=${encodeURIComponent(file.path)}`}
              download
            >
              {file.path}
            </a>{' '}
            · {file.kind} · {file.bytes.toLocaleString()} bytes
            <p>{file.description}</p>
            {file.claims.length ? (
              <p className="small">Supports: {file.claims.join(', ')}</p>
            ) : null}
            {file.source_url && safeHttpUrl(file.source_url) ? (
              <p className="small">
                <a href={file.source_url} target="_blank" rel="noopener noreferrer">
                  Reported source
                </a>
              </p>
            ) : null}
            {technicalDetails && file.source_revision ? (
              <p className="small">
                Source revision: <code>{file.source_revision}</code>
              </p>
            ) : null}
            {technicalDetails ? (
              <details>
                <summary>File fingerprint</summary>
                <code className="evidence-hash">SHA-256 {file.sha256}</code>
              </details>
            ) : null}
          </li>
        ))}
      </ul>
    </section>
  );
}
