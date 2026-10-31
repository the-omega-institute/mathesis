import type { PublicPaper } from '../api/types';
import { sourceFileHref } from '../api/client';
import { safeHttpUrl } from '../lib/links';
import { Link } from '../routing/router';

export function Bibliography({ paper }: { paper: PublicPaper }) {
  if (!paper.publication) return null;
  const bibliography = paper.bibliography;
  const version = paper.publication.version;
  return (
    <section aria-labelledby="references-h" className="paper-references">
      <h2 id="references-h">References and connections</h2>
      <p>
        <Link to={{ kind: 'research_graph', focus: `paper:${paper.summary.record}` }}>
          Explore this paper in the research graph
        </Link>
        {' · '}
        <Link to={{ kind: 'research_graph' }}>Browse the shared graph</Link>
      </p>
      <p className="muted">
        Citations credit the literature named in the manuscript. They do not establish a checked
        mathematical dependency or award proof credit.
      </p>
      {!bibliography ? (
        <p>This accepted source version has not been indexed. Consult the original references.</p>
      ) : (
        <>
          {!bibliography.references.length ? (
            <p>No literal citations were indexed from this version. Coverage may be incomplete.</p>
          ) : (
            <ol className="reference-list">
              {bibliography.references.map(({ work, display_text, reference: r }) => {
                const href = r.url ? safeHttpUrl(r.url) : null;
                const citationText =
                  r.title?.trim() || display_text || 'Cited work described in the manuscript';
                return (
                  <li key={r.key}>
                    <Link to={{ kind: 'research_graph', focus: work }}>{citationText}</Link>
                    {r.authors.length ? (
                      <p>
                        {r.authors.join('; ')}
                        {r.year ? ` (${r.year})` : ''}
                      </p>
                    ) : null}
                    <p className="entry-meta">Cited earlier work · source-declared relationship.</p>
                    {r.doi ? <p>DOI as supplied: {r.doi}</p> : null}
                    {r.arxiv ? <p>arXiv as supplied: {r.arxiv}</p> : null}
                    {href ? (
                      <a href={href} target="_blank" rel="noreferrer">
                        Source link
                      </a>
                    ) : null}
                    <details>
                      <summary>Source evidence</summary>
                      <pre className="reference-evidence">{r.raw}</pre>
                      {r.raw_truncated ? <p>Only a bounded prefix is shown.</p> : null}
                      <p>
                        Reference key {r.key} · version {version} · {r.source_location}
                        {paper.summary.submission ? (
                          <>
                            {' '}
                            ·{' '}
                            <a href={sourceFileHref(paper.summary.submission, version)}>
                              Original upload
                            </a>
                          </>
                        ) : null}
                      </p>
                    </details>
                  </li>
                );
              })}
            </ol>
          )}
          <details>
            <summary>Reference coverage</summary>
            <ul>
              {bibliography.limitations.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
          </details>
        </>
      )}
    </section>
  );
}
