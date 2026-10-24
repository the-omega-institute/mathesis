import { InlineLatex } from '../components/Markdown';
import { ProofAttribution } from '../components/ProofAttribution';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { EntrantName } from './Leaderboard';
import { useCallback, useMemo, useState } from 'react';
import { pdfHref, type PaperQuery } from '../api/client';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { usePaged } from '../api/usePaged';
import type { Author, PaperSummary, PublicPaper } from '../api/types';
import { AuthorLine } from '../components/AuthorLine';
import { LeanBadge } from '../components/badges';
import { claimAnchor } from '../components/claims';
import { statementName } from '../components/StatementSummary';
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
      <p className="entry-meta">
        {paper.new_results ? <span>{newResults(paper.new_results)}</span> : null}
        {paper.lean_verified ? <Badge tone="good">Lean ✓ {paper.lean_verified}</Badge> : null}
        <MscList codes={paper.msc ?? []} />
      </p>
    </article>
  );
}

export function PapersPage() {
  const api = useApi();
  const [searchScope, setSearchScope] = useState<'research' | 'authors'>('research');
  const [query, setQuery] = useState<PaperQuery>({});
  const [draft, setDraft] = useState<PaperQuery>({});
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      searchScope === 'authors'
        ? query.q?.trim()
          ? api.listPapers({ author: query.q.trim(), before, limit: 100 }, { signal })
          : Promise.resolve({ items: [], next_before: null })
        : api.listPapers({ ...query, before, limit: 25 }, { signal }),
    [api, query, searchScope],
  );
  const paged = usePaged(fetchPage);
  const authorResults = useMemo(
    () => collectAuthors(paged.value?.items ?? [], query.q ?? ''),
    [paged.value, query.q],
  );
  const hasAuthorQuery = searchScope === 'authors' && Boolean(query.q?.trim());
  return (
    <div className="page discovery-page">
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
        aria-label="Search research archive"
        onSubmit={(event) => {
          event.preventDefault();
          setQuery(searchScope === 'authors' ? { q: draft.q?.trim() ?? '' } : { ...draft });
        }}
      >
        <label className="archive-search-scope">
          Search in
          <select
            value={searchScope}
            onChange={(event) => {
              const scope = event.target.value as 'research' | 'authors';
              setSearchScope(scope);
              setDraft({});
              setQuery({});
            }}
          >
            <option value="research">Research archive</option>
            <option value="authors">Authors</option>
          </select>
        </label>
        <div className="archive-search-main">
          <label htmlFor="paper-query">
            {searchScope === 'authors' ? 'Find an author' : 'Find research'}
          </label>
          <div className="archive-search-input">
            <input
              id="paper-query"
              type="search"
              maxLength={300}
              placeholder={
                searchScope === 'authors'
                  ? 'Author name'
                  : 'Title, abstract, keywords or WP record…'
              }
              value={draft.q ?? ''}
              onChange={(e) => setDraft({ ...draft, q: e.target.value })}
            />
            <button className="button" type="submit">
              Search
            </button>
          </div>
        </div>
        {searchScope === 'research' ? (
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
          </div>
        ) : null}
        <div className="archive-search-actions">
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
          <p className="hint">
            {searchScope === 'authors'
              ? 'Names are drawn from public research records.'
              : 'Search public titles and abstracts across the archive. Results are ordered newest first.'}
          </p>
        </div>
      </form>
      <Async state={paged.state} onRetry={paged.reload}>
        {(listing) => {
          if (searchScope === 'authors') {
            if (!hasAuthorQuery) {
              return <p className="muted">Search by name to find authors in the archive.</p>;
            }
            return authorResults.length === 0 ? (
              <p className="muted">No authors match this search. Try a broader name.</p>
            ) : (
              <ul className="entry-list author-results">
                {authorResults.map(({ author, papers }) => (
                  <li key={authorKey(author)}>
                    <Link className="entry-title" to={{ kind: 'author', name: author.name }}>
                      {author.name}
                    </Link>
                    {author.affiliation ? <p className="entry-meta">{author.affiliation}</p> : null}
                    <p className="entry-meta">
                      {papers} {papers === 1 ? 'public record' : 'public records'}
                    </p>
                  </li>
                ))}
              </ul>
            );
          }
          return listing.items.length === 0 ? (
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
          );
        }}
      </Async>
      {searchScope === 'research' ? <Pager paged={paged} /> : null}
    </div>
  );
}

function authorKey(author: Author): string {
  return author.name.trim().toLocaleLowerCase();
}

function collectAuthors(
  papers: readonly PaperSummary[],
  search: string,
): { author: Author; papers: number }[] {
  const needle = search.trim().toLocaleLowerCase();
  const results = new Map<string, { author: Author; papers: number }>();
  for (const paper of papers) {
    for (const author of paper.authors) {
      if (!author.name.toLocaleLowerCase().includes(needle)) continue;
      const key = authorKey(author);
      const existing = results.get(key);
      if (existing) existing.papers += 1;
      else results.set(key, { author, papers: 1 });
    }
  }
  return [...results.values()].sort((a, b) => a.author.name.localeCompare(b.author.name));
}

export function PaperPage({ record }: { record: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.getPaper(record, { signal }),
    [api, record],
  );
  const paper = useAsync(load);
  return (
    <div className="page reading-page">
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
        <Citation paper={s} />
        {claims.length > 0 ? (
          <section aria-labelledby="statements-h" className="public-results">
            <h2 id="statements-h">Statements</h2>
            {[...claims]
              .sort((a, b) => Number(b.role === 'main') - Number(a.role === 'main'))
              .map((claim) => {
                const content = paper.new_content?.find((item) => item.claim === claim.id);
                const target = paper.lean_statements?.find((item) => item.claim === claim.id);
                return (
                  <article
                    className="public-result"
                    id={claimAnchor(claim.id, 'public')}
                    key={claim.id}
                    aria-labelledby={`result-${claim.id}`}
                  >
                    <div className="public-result-heading">
                      <div>
                        <p className="eyebrow">
                          {CLAIM_KIND_LABELS[claim.kind]} · <span>{claim.id}</span> ·{' '}
                          {claim.role === 'main' ? 'Main result' : 'Supporting statement'}
                        </p>
                        <h3 id={`result-${claim.id}`}>
                          <InlineLatex source={statementName(claim) ?? claim.label} />
                        </h3>
                      </div>
                      {claim.lean || claim.verified ? <Badge tone="good">Lean ✓</Badge> : null}
                    </div>
                    <LatexText source={claim.statement} className="statement" />
                    <p className="small result-authors">
                      <strong>Authors:</strong> {s.authors.map((author) => author.name).join(', ')}
                    </p>
                    {content?.lemmas.length ? (
                      <div className="result-contribution">
                        <h4>Mathematical contribution</h4>
                        <ul>
                          {content.lemmas.map((lemma, index) => (
                            <li key={index}>
                              <LatexText source={lemma} />
                            </li>
                          ))}
                        </ul>
                      </div>
                    ) : null}
                    {claim.verified ? (
                      <p className="small">
                        <strong>Formalization:</strong>{' '}
                        <EntrantName entrant={claim.verified.entrant} /> proved the author-confirmed
                        Lean target.
                      </p>
                    ) : null}
                    {claim.lean ? (
                      <p className="small">
                        <LeanBadge artifact={claim.lean} />
                      </p>
                    ) : null}
                    <ProofAttribution attribution={claim.verified?.attribution} />
                    <div className="result-links">
                      <Link to={{ kind: 'statement', record: s.record, claim: claim.id }}>
                        Formalization and proofs
                      </Link>
                    </div>
                    <details className="result-details">
                      <summary>Statement details</summary>
                      {claim.depends_on.length ? (
                        <p className="small muted">Uses {claim.depends_on.join(', ')}.</p>
                      ) : null}
                      {target ? (
                        <section aria-label="Author-confirmed Lean statement">
                          <h4>Author-confirmed Lean statement</h4>
                          <pre className="lean-source">
                            <code>{target.lean}</code>
                          </pre>
                          <ReportError
                            record={s.record}
                            subject={{ kind: 'target', claim: target.claim, digest: target.digest }}
                          />
                        </section>
                      ) : null}
                      <ReportError
                        record={s.record}
                        subject={{ kind: 'statement', claim: claim.id }}
                      />
                    </details>
                  </article>
                );
              })}
          </section>
        ) : (
          <p className="muted">The author keeps the mathematical details private.</p>
        )}

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
