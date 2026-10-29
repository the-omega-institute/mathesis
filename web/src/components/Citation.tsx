import { useState } from 'react';
import type { PaperSummary } from '../api/types';
import { paperBibtex, paperCitation, type CitationLocator } from '../lib/citation';

export function Citation({
  paper,
  locator = {},
}: {
  paper: PaperSummary;
  locator?: CitationLocator;
}) {
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState(false);
  const [format, setFormat] = useState<'text' | 'bibtex'>('text');
  const text = paperCitation(paper, window.location.origin, locator);
  const bibtex = paperBibtex(paper, window.location.origin, locator);
  const displayed = format === 'text' ? text : bibtex;
  const download = () => {
    const url = URL.createObjectURL(
      new Blob([bibtex], { type: 'application/x-bibtex;charset=utf-8' }),
    );
    const link = document.createElement('a');
    link.href = url;
    link.download = `${paper.record}${locator.claim ? `-${locator.claim.id}` : ''}.bib`;
    link.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  };
  return (
    <details className="citation" id="citation">
      <summary>{locator.claim ? 'Cite this result' : 'Cite this paper'}</summary>
      <label className="citation-format">
        Citation format
        <select
          value={format}
          onChange={(event) => {
            setFormat(event.target.value as 'text' | 'bibtex');
            setCopied(false);
            setError(false);
          }}
        >
          <option value="text">Plain text</option>
          <option value="bibtex">BibTeX</option>
        </select>
      </label>
      {format === 'text' ? (
        <p className="cite">{displayed}</p>
      ) : (
        <pre className="citation-text">
          <code>{displayed}</code>
        </pre>
      )}
      <div className="button-row">
        {navigator.clipboard ? (
          <button
            type="button"
            className="button button-quiet button-small"
            onClick={() =>
              navigator.clipboard.writeText(displayed).then(
                () => {
                  setCopied(true);
                  setError(false);
                },
                () => setError(true),
              )
            }
          >
            {copied ? 'Copied' : 'Copy citation'}
          </button>
        ) : null}
        <button type="button" className="button button-quiet button-small" onClick={download}>
          Download BibTeX
        </button>
      </div>
      {error ? (
        <p role="status" className="small">
          Copying is unavailable. Select the citation above, or download the BibTeX file.
        </p>
      ) : null}
    </details>
  );
}
