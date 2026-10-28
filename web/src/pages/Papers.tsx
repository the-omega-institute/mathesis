import { InlineLatex } from '../components/Markdown';
import { ProofAttribution } from '../components/ProofAttribution';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { EntrantName } from './Leaderboard';
import { useCallback, useState } from 'react';
import { pdfHref, type PaperQuery } from '../api/client';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { usePaged } from '../api/usePaged';
import type { PaperSummary, PublicPaper } from '../api/types';
import { AuthorLine } from '../components/AuthorLine';
import { PublicStatements, statementName } from '../components/StatementSummary';
import { LatexText, MacrosProvider } from '../components/Markdown';
import { Pager } from '../components/Pager';
import { Async, Badge, DateText, ExternalLink, MscList } from '../components/ui';
import { paperCitation } from '../lib/citation';
import { CLAIM_KIND_LABELS, SUBMISSION_KIND_LABELS, newResults } from '../lib/labels';
import { safeHttpUrl, sourceHref } from '../lib/links';
import { Link } from '../routing/router';

export function PaperEntry({ paper }: { paper: PaperSummary }) {
  return (
    <article className="paper-entry">
      <p className="entry-title">
        <Link to={{ kind: 'paper', record: paper.record }}>
          <InlineLatex source={paper.title} />
        </Link>
      </p>
      <div className="entry-meta">
        <AuthorLine authors={paper.authors} />
        <p>
          · <code>{paper.record}</code> · {SUBMISSION_KIND_LABELS[paper.kind]}
        </p>
        <p>
          {paper.accepted_at ? (
            <>
              · accepted <DateText iso={paper.accepted_at} />
            </>
          ) : null}
        </p>
      </div>
      {paper.abstract_text ? (
        <LatexText source={paper.abstract_text} className="abstract-excerpt" />
      ) : null}
      <p className="entry-meta">
        {paper.new_results ? <span>{newResults(paper.new_results)}</span> : null}
        {paper.lean_verified ? <Badge tone="good">Lean ✓ {paper.lean_verified}</Badge> : null}
        <MscList codes={paper.msc ?? []} />
      </p>
    </article>
  );
}

export function PapersPage({ initialQuery = '' }: { initialQuery?: string } = {}) {
  const api = useApi();
  const [query, setQuery] = useState<PaperQuery>(initialQuery ? { q: initialQuery } : {});
  const [draft, setDraft] = useState<PaperQuery>(initialQuery ? { q: initialQuery } : {});
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.listPapers({ ...query, before, limit: 25 }, { signal }),
    [api, query],
  );
  const paged = usePaged(fetchPage);
  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>Research archive</h1>
          <p className="lede">
            Papers and notes with new mathematical content, and conjectures accepted for display.{' '}
            <Link to={{ kind: 'policy' }}>Review policy</Link>
          </p>
        </div>
      </header>
      <form
        className="archive-search"
        role="search"
        aria-label="Search papers"
        onSubmit={(event) => {
          event.preventDefault();
          setQuery({ ...draft });
        }}
      >
        <div className="archive-search-main">
          <label htmlFor="paper-query">Find research</label>
          <div className="archive-search-input">
            <input
              id="paper-query"
              type="search"
              maxLength={300}
              placeholder="Title, author, abstract, DOI or record…"
              value={draft.q ?? ''}
              onChange={(e) => setDraft({ ...draft, q: e.target.value })}
            />
            <button className="button" type="submit">
              Search
            </button>
          </div>
        </div>
        <details className="archive-search-advanced">
          <summary>Advanced search</summary>
          <div className="archive-search-filters">
            <label>
              Author
              <input
                maxLength={160}
                placeholder="Author name"
                value={draft.author ?? ''}
                onChange={(e) => setDraft({ ...draft, author: e.target.value })}
              />
            </label>
            <label>
              Subject (MSC)
              <input
                maxLength={5}
                placeholder="e.g. 11 or 11A"
                value={draft.msc ?? ''}
                onChange={(e) => setDraft({ ...draft, msc: e.target.value })}
              />
            </label>
            <label>
              Type
              <select
                value={draft.kind ?? ''}
                onChange={(e) => setDraft({ ...draft, kind: e.target.value as PaperQuery['kind'] })}
              >
                <option value="">All research</option>
                <option value="paper">Papers</option>
                <option value="note">Short notes</option>
                <option value="conjecture">Conjectures</option>
              </select>
            </label>
            <button
              className="button button-quiet"
              type="button"
              onClick={() => {
                setDraft({});
                setQuery({});
              }}
            >
              Clear filters
            </button>
          </div>
        </details>
        <p className="hint">
          Search papers, notes and conjectures by title, author, abstract or identifier.
        </p>
      </form>
      <Async state={paged.state} onRetry={paged.reload}>
        {(listing) =>
          listing.items.length === 0 ? (
            <p className="muted">
              No papers match this view. Try a broader search or clear the filters.
            </p>
          ) : (
            <ul className="entry-list">
              {listing.items.map((p) => (
                <li key={p.record}>
                  <PaperEntry paper={p} />
                </li>
              ))}
            </ul>
          )
        }
      </Async>
      <Pager paged={paged} />
    </div>
  );
}

export function PaperPage({ record }: { record: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.getPaper(record, { signal }),
    [api, record],
  );
  const paper = useAsync(load);
  return (
    <div className="page">
      <Async state={paper.state} onRetry={paper.reload}>
        {(p) => <PaperView paper={p} />}
      </Async>
    </div>
  );
}

export function PaperView({ paper }: { paper: PublicPaper }) {
  const s = paper.summary;
  const latest = paper.versions?.at(-1);
  const claims = paper.claims ?? [];
  const repo = paper.formalization_repository ? safeHttpUrl(paper.formalization_repository) : null;
  const arxiv = /^10\.48550\/arxiv\.(.+)$/i.exec(s.doi ?? '')?.[1];
  const doiHref = s.doi ? sourceHref({ kind: 'doi', locator: s.doi }) : null;
  return (
    <MacrosProvider macros={paper.macros ?? {}}>
      <article className="paper">
        <header className="record-head">
          <p className="eyebrow">
            Record <code className="record-id">{s.record}</code> · {SUBMISSION_KIND_LABELS[s.kind]}
          </p>
          <h1>
            <InlineLatex source={s.title} />
          </h1>
          <AuthorLine authors={s.authors} />
          {paper.confirmed_error_reports ? (
            <p className="notice" role="status">
              This published version has {paper.confirmed_error_reports} confirmed error report(s).
              Read the error reports before relying on its results. The original acceptance record
              is preserved; acceptance is not a proof of correctness.
            </p>
          ) : null}
          {claims.length ? <ReportError record={s.record} subject={{ kind: 'review' }} /> : null}
          <p className="entry-meta">
            {s.accepted_at ? (
              <span>
                {s.kind === 'conjecture' ? 'Displayed' : 'Accepted'}{' '}
                <DateText iso={s.accepted_at} />
              </span>
            ) : null}
            <MscList codes={s.msc ?? []} />
            {arxiv ? <span>arXiv:{arxiv}</span> : null}
            {s.doi ? (
              <span>
                · {doiHref ? <ExternalLink href={doiHref}>doi:{s.doi}</ExternalLink> : s.doi}
              </span>
            ) : null}
          </p>
          <div className="button-row">
            {latest?.has_pdf && s.submission ? (
              <a className="button" href={pdfHref(s.submission, latest.number)}>
                PDF (version {latest.number})
              </a>
            ) : null}
            {repo ? <ExternalLink href={repo}>Lean formalization repository</ExternalLink> : null}
          </div>
        </header>

        {s.abstract_text ? (
          <section aria-labelledby="abstract-h" className="paper-abstract">
            <h2 id="abstract-h">Abstract</h2>
            <LatexText source={s.abstract_text} className="abstract" />
          </section>
        ) : null}
        {paper.versions?.length ? (
          <section aria-labelledby="public-versions-h">
            <h2 id="public-versions-h">Version history</h2>
            <ol className="version-list">
              {[...paper.versions].reverse().map((version) => (
                <li key={version.number}>
                  <strong>Version {version.number}</strong> ·{' '}
                  <DateText iso={version.uploaded_at} withTime />
                  {version.has_pdf && s.submission ? (
                    <>
                      {' '}
                      ·{' '}
                      <a href={pdfHref(s.submission, version.number)}>
                        Version {version.number} PDF
                      </a>
                    </>
                  ) : null}
                </li>
              ))}
            </ol>
          </section>
        ) : null}
        <Citation paper={s} />
        {paper.new_content?.length ? (
          <section aria-labelledby="new-content-h" className="new-content">
            <h2 id="new-content-h">
              {s.kind === 'note' ? 'New in this note' : 'New in this paper'}
            </h2>
            <ul className="new-content-list">
              {paper.new_content.map((w) => {
                const claim = claims.find((c) => c.id === w.claim);
                return (
                  <li key={w.claim}>
                    <p className="new-content-claim">
                      <span className="statement-kind">
                        {claim ? CLAIM_KIND_LABELS[claim.kind] : 'Statement'}{' '}
                        <span className="statement-id">{w.claim}</span>
                      </span>{' '}
                      {claim ? (
                        <LatexText
                          source={statementName(claim) ?? claim.statement}
                          className="statement-preview"
                        />
                      ) : null}
                    </p>
                    <ul className="new-content-lemmas">
                      {w.lemmas.map((lemma, i) => (
                        <li key={i}>
                          <LatexText source={lemma} />
                        </li>
                      ))}
                    </ul>
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}
        {claims.length > 0 ? (
          <section aria-labelledby="statements-h" className="statements">
            <h2 id="statements-h">Statements</h2>
            <PublicStatements claims={claims} />
            <ul className="entry-list">
              {claims.map((claim) => (
                <li key={claim.id}>
                  <Link to={{ kind: 'statement', record: s.record, claim: claim.id }}>
                    {claim.label} · Lean target and attempts
                  </Link>
                  {claim.verified ? (
                    <p>
                      <strong>Lean ✓</strong> · <EntrantName entrant={claim.verified.entrant} />{' '}
                      proved the author-confirmed Lean target.
                    </p>
                  ) : null}
                  <ProofAttribution attribution={claim.verified?.attribution} />
                  <ReportError record={s.record} subject={{ kind: 'statement', claim: claim.id }} />
                </li>
              ))}
            </ul>
          </section>
        ) : (
          <p className="muted">The author keeps the mathematical details private.</p>
        )}
        {paper.lean_statements?.map((target) => (
          <section key={target.digest} aria-label="Author-confirmed Lean statement">
            <h2>Author-confirmed Lean statement</h2>
            <pre className="lean-source">
              <code>{target.lean}</code>
            </pre>
            <ReportError
              record={s.record}
              subject={{ kind: 'target', claim: target.claim, digest: target.digest }}
            />
          </section>
        ))}

        {claims.length ? <ErrorReports record={s.record} /> : null}
      </article>
    </MacrosProvider>
  );
}

function Citation({ paper }: { paper: PaperSummary }) {
  const [copied, setCopied] = useState(false);
  const text = paperCitation(paper, window.location.origin);
  const canCopy = typeof navigator !== 'undefined' && Boolean(navigator.clipboard);
  return (
    <section aria-labelledby="cite-h" className="citation">
      <h2 id="cite-h">Cite</h2>
      <p className="cite">{text}</p>
      {canCopy ? (
        <button
          type="button"
          className="button button-quiet button-small"
          onClick={() =>
            navigator.clipboard.writeText(text).then(
              () => setCopied(true),
              () => setCopied(false),
            )
          }
        >
          {copied ? 'Copied' : 'Copy citation'}
        </button>
      ) : null}
    </section>
  );
}
