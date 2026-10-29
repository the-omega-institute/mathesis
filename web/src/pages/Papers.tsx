import { InlineLatex } from '../components/Markdown';
import { ManuscriptSources } from '../components/ManuscriptSources';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { useCallback, useState } from 'react';
import { pdfHref, sourceFileHref, type PaperQuery } from '../api/client';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { usePaged } from '../api/usePaged';
import type { PaperSummary, PublicPaper } from '../api/types';
import { AuthorLine } from '../components/AuthorLine';
import { Citation } from '../components/Citation';
import { PaperMetadata } from '../components/PaperMetadata';
import { PaperResults } from '../components/PaperResults';
import { LatexText, MacrosProvider } from '../components/Markdown';
import { Pager } from '../components/Pager';
import { Async, Badge, DateText, ExternalLink, MscList } from '../components/ui';
import { SUBMISSION_KIND_LABELS, newResults } from '../lib/labels';
import { safeHttpUrl, sourceHref } from '../lib/links';
import { Link, useRouter } from '../routing/router';

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

export function PapersPage({
  initialQuery = '',
  initialFilters = {},
}: { initialQuery?: string; initialFilters?: Pick<PaperQuery, 'author' | 'msc' | 'kind'> } = {}) {
  const api = useApi();
  const { navigate } = useRouter();
  const [query, setQuery] = useState<PaperQuery>({
    ...initialFilters,
    ...(initialQuery ? { q: initialQuery } : {}),
  });
  const [draft, setDraft] = useState<PaperQuery>(query);
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
          navigate({
            kind: 'papers',
            q: draft.q?.trim(),
            author: draft.author?.trim(),
            msc: draft.msc?.trim(),
            work: draft.kind || undefined,
          });
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
        <details
          className="archive-search-advanced"
          open={
            Boolean(initialFilters.author || initialFilters.msc || initialFilters.kind) || undefined
          }
        >
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
                navigate({ kind: 'papers' });
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
  const versions = paper.versions ?? [];
  const latest = versions.at(-1);
  const claims = paper.claims ?? [];
  const repo = paper.formalization_repository ? safeHttpUrl(paper.formalization_repository) : null;
  const arxiv = /^10\.48550\/arxiv\.(.+)$/i.exec(s.doi ?? '')?.[1];
  const doiHref = s.doi ? sourceHref({ kind: 'doi', locator: s.doi }) : null;
  return (
    <MacrosProvider macros={paper.macros ?? {}}>
      <article className="paper academic-paper">
        <PaperMetadata paper={paper} />
        <p className="entry-meta">
          <Link to={{ kind: 'papers' }}>Research archive</Link> · {s.record} ·{' '}
          {SUBMISSION_KIND_LABELS[s.kind]}
        </p>
        <header className="record-head">
          <h1>
            <InlineLatex source={s.title} />
          </h1>
          <AuthorLine authors={s.authors} />
          <p className="entry-meta">
            {versions[0] ? (
              <>
                First version <DateText iso={versions[0].uploaded_at} /> ·{' '}
              </>
            ) : null}
            {s.accepted_at ? (
              <>
                Accepted <DateText iso={s.accepted_at} />
              </>
            ) : null}
            {latest ? ` · version ${latest.number}` : null}
          </p>
          <nav className="paper-access" aria-label="Paper files and citation">
            {latest?.has_pdf && s.submission ? (
              <a href={pdfHref(s.submission, latest.number)}>View PDF</a>
            ) : null}
            {latest && s.submission && !latest.source_is_pdf ? (
              <a href={sourceFileHref(s.submission, latest.number)}>
                {latest.source_is_pdf === false ? 'Download source' : 'Original upload'}
              </a>
            ) : null}
            <a
              href="#citation"
              onClick={() => document.getElementById('citation')?.setAttribute('open', '')}
            >
              Cite
            </a>
            {repo ? <ExternalLink href={repo}>Formal proofs</ExternalLink> : null}
          </nav>
        </header>
        {paper.confirmed_error_reports ? (
          <p className="notice" role="status">
            This version has {paper.confirmed_error_reports} confirmed error report(s). Read the
            corrections before relying on its results.
          </p>
        ) : null}
        {s.abstract_text ? (
          <section aria-labelledby="abstract-h" className="paper-abstract">
            <h2 id="abstract-h">Abstract</h2>
            <LatexText source={s.abstract_text} className="abstract" />
          </section>
        ) : null}
        {s.msc?.length || s.doi ? (
          <dl className="paper-metadata">
            {s.msc?.length ? (
              <>
                <dt>Subjects</dt>
                <dd>
                  {s.msc.map((code, index) => (
                    <span key={code}>
                      {index ? '; ' : ''}
                      <Link to={{ kind: 'papers', msc: code }}>MSC {code}</Link>
                    </span>
                  ))}
                </dd>
              </>
            ) : null}
            {s.doi ? (
              <>
                <dt>{arxiv ? 'arXiv' : 'DOI'}</dt>
                <dd>
                  {doiHref ? (
                    <ExternalLink
                      href={arxiv ? `https://arxiv.org/abs/${encodeURIComponent(arxiv)}` : doiHref}
                    >
                      {arxiv ? `arXiv:${arxiv}` : s.doi}
                    </ExternalLink>
                  ) : (
                    s.doi
                  )}
                </dd>
              </>
            ) : null}
          </dl>
        ) : null}
        {versions.length ? (
          <section aria-labelledby="public-versions-h" className="paper-versions">
            <h2 id="public-versions-h">Version history</h2>
            <ol className="version-list">
              {[...versions].reverse().map((version) => (
                <li key={version.number}>
                  <strong>Version {version.number}</strong> · <DateText iso={version.uploaded_at} />
                  {version.has_pdf && s.submission ? (
                    <>
                      {' '}
                      ·{' '}
                      <a href={pdfHref(s.submission, version.number)}>
                        Version {version.number} PDF
                      </a>
                    </>
                  ) : null}
                  {s.submission && !version.source_is_pdf ? (
                    <>
                      {' '}
                      ·{' '}
                      <a href={sourceFileHref(s.submission, version.number)}>
                        Version {version.number} source
                      </a>
                    </>
                  ) : null}
                </li>
              ))}
            </ol>
          </section>
        ) : null}
        <Citation paper={s} locator={{ version: paper.publication?.version ?? latest?.number }} />
        {paper.origin ? <ManuscriptSources origin={paper.origin} /> : null}
        {claims.length ? (
          <PaperResults paper={paper} />
        ) : (
          <p className="muted">The author keeps the mathematical details private.</p>
        )}
        {claims.length ? (
          <details className="history">
            <summary>Corrections</summary>
            <ReportError record={s.record} subject={{ kind: 'review' }} />
            <ErrorReports record={s.record} />
          </details>
        ) : null}
      </article>
    </MacrosProvider>
  );
}
