import { t } from '../i18n/locale';
import { PaperExplanation } from '../components/PaperExplanation';
import { findProjectNotes, projectNotes } from '../lib/projectNotes';
import { researchEntries } from '../lib/researchEntries';
import { PublishedNoteEntry, PublishedNotePage } from './PublishedNote';
import { ProblemLinks } from './ResearchProblems';
import { Bibliography } from '../components/Bibliography';
import { InlineLatex } from '../components/Markdown';
import { ManuscriptSources } from '../components/ManuscriptSources';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { useCallback, useState } from 'react';
import { pdfHref, type PaperQuery } from '../api/client';
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
import { Async, DateText, ExternalLink, MscList } from '../components/ui';
import { SUBMISSION_KIND_LABELS } from '../lib/labels';
import { safeHttpUrl, sourceHref } from '../lib/links';
import { Link, useRouter } from '../routing/router';

export function PaperEntry({ paper }: { paper: PaperSummary }) {
  return (
    <article className="paper-entry research-entry">
      <h3>
        <Link to={{ kind: 'paper', record: paper.record }}>
          <InlineLatex source={paper.title} />
        </Link>
      </h3>
      <AuthorLine authors={paper.authors.map(({ name }) => ({ name }))} />
      <div className="entry-meta">
        <span>{paper.record}</span>
        <span>{t(SUBMISSION_KIND_LABELS[paper.kind])}</span>
        {paper.accepted_at ? <DateText iso={paper.accepted_at} /> : null}
        <MscList codes={paper.msc ?? []} />
      </div>
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
  // A locally published manuscript appears once, on the first archive page.
  // Following the API cursor must not repeat it on every subsequent page.
  const notes = paged.page === 1 ? findProjectNotes(query) : [];
  const entries = researchEntries(paged.value?.items ?? [], notes);
  return (
    <div className="page research-archive">
      <header className="page-head">
        <div>
          <h1>{t('Research archive')}</h1>
          <p className="lede">
            {t('Papers, short notes and conjectures, with their sources and formal proofs.')}{' '}
            <Link to={{ kind: 'policy' }}>{t('Review policy')}</Link>
          </p>
        </div>
      </header>
      <form
        className="archive-search"
        role="search"
        aria-label={t('Search papers')}
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
          <label htmlFor="paper-query">{t('Find research')}</label>
          <div className="archive-search-input">
            <input
              id="paper-query"
              type="search"
              maxLength={300}
              placeholder={t('Title, author, abstract, DOI or record…')}
              value={draft.q ?? ''}
              onChange={(e) => setDraft({ ...draft, q: e.target.value })}
            />
            <button className="button" type="submit">
              {t('Search')}
            </button>
          </div>
        </div>
        <details
          className="archive-search-advanced"
          open={
            Boolean(initialFilters.author || initialFilters.msc || initialFilters.kind) || undefined
          }
        >
          <summary>{t('Advanced search')}</summary>
          <div className="archive-search-filters">
            <label>
              {t('Author')}
              <input
                maxLength={160}
                placeholder={t('Author name')}
                value={draft.author ?? ''}
                onChange={(e) => setDraft({ ...draft, author: e.target.value })}
              />
            </label>
            <label>
              {t('Subject (MSC)')}
              <input
                maxLength={5}
                placeholder={t('e.g. 11 or 11A')}
                value={draft.msc ?? ''}
                onChange={(e) => setDraft({ ...draft, msc: e.target.value })}
              />
            </label>
            <label>
              {t('Type')}
              <select
                value={draft.kind ?? ''}
                onChange={(e) => setDraft({ ...draft, kind: e.target.value as PaperQuery['kind'] })}
              >
                <option value="">{t('All research')}</option>
                <option value="paper">{t('Papers')}</option>
                <option value="note">{t('Short notes')}</option>
                <option value="conjecture">{t('Conjectures')}</option>
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
              {t('Clear filters')}
            </button>
          </div>
        </details>
        <p className="hint">
          {t('Search papers, notes and conjectures by title, author, abstract or identifier.')}
        </p>
      </form>
      {entries.length ? (
        <ol className="entry-list research-archive-list">
          {entries.map((entry) =>
            entry.kind === 'note' ? (
              <li key={`note:${entry.note.id}`}>
                <PublishedNoteEntry note={entry.note} />
              </li>
            ) : (
              <li key={`paper:${entry.paper.record}`}>
                <PaperEntry paper={entry.paper} />
              </li>
            ),
          )}
        </ol>
      ) : null}
      <Async state={paged.state} onRetry={paged.reload}>
        {(listing) =>
          listing.items.length === 0 && notes.length === 0 ? (
            <p className="muted">
              {t('No papers match this view. Try a broader search or clear the filters.')}
            </p>
          ) : null
        }
      </Async>
      <Pager paged={paged} />
    </div>
  );
}

export function PaperPage({ record, version }: { record: string; version?: string }) {
  if (projectNotes.some((note) => note.id === record)) {
    return <PublishedNotePage id={record} version={version} />;
  }
  return <ReviewedPaperPage record={record} />;
}

function ReviewedPaperPage({ record }: { record: string }) {
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
          <Link to={{ kind: 'papers' }}>{t('Research archive')}</Link> · {s.record} ·{' '}
          {t(SUBMISSION_KIND_LABELS[s.kind])}
        </p>
        <header className="record-head">
          <h1>
            <InlineLatex source={s.title} />
          </h1>
          <AuthorLine authors={s.authors} />
          <p className="entry-meta">
            {versions[0] ? (
              <>
                {t('First version ')}
                <DateText iso={versions[0].uploaded_at} /> ·{' '}
              </>
            ) : null}
            {s.accepted_at ? (
              <>
                {t('Accepted ')}
                <DateText iso={s.accepted_at} />
              </>
            ) : null}
            {latest ? ` · version ${latest.number}` : null}
          </p>
          <nav className="paper-access" aria-label={t('Paper files and citation')}>
            {latest?.has_pdf && s.submission ? (
              <a href={pdfHref(s.submission, latest.number)}>{t('View PDF')}</a>
            ) : null}
            <a
              href="#citation"
              onClick={() => document.getElementById('citation')?.setAttribute('open', '')}
            >
              {t('Cite')}
            </a>
            {repo ? <ExternalLink href={repo}>{t('Formal proofs')}</ExternalLink> : null}
          </nav>
        </header>
        {paper.confirmed_error_reports ? (
          <p className="notice" role="status">
            {t('This version has ')}
            {paper.confirmed_error_reports}{' '}
            {t(' confirmed error report(s). Read the corrections before relying on its results.')}
          </p>
        ) : null}
        <PaperExplanation text={paper.plain_language} />
        {s.abstract_text ? (
          <section aria-labelledby="abstract-h" className="paper-abstract">
            <h2 id="abstract-h">{t('Abstract')}</h2>
            <LatexText source={s.abstract_text} className="abstract" />
          </section>
        ) : null}
        {s.msc?.length || s.doi ? (
          <dl className="paper-metadata">
            {s.msc?.length ? (
              <>
                <dt>{t('Subjects')}</dt>
                <dd>
                  {s.msc.map((code, index) => (
                    <span key={code}>
                      {index ? '; ' : ''}
                      <Link to={{ kind: 'papers', msc: code }}>
                        {t('MSC ')}
                        {code}
                      </Link>
                    </span>
                  ))}
                </dd>
              </>
            ) : null}
            {s.doi ? (
              <>
                <dt>{arxiv ? t('arXiv') : t('DOI')}</dt>
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
            <h2 id="public-versions-h">{t('Version history')}</h2>
            <ol className="version-list">
              {[...versions].reverse().map((version) => (
                <li key={version.number}>
                  <strong>
                    {t('Version ')}
                    {version.number}
                  </strong>{' '}
                  · <DateText iso={version.uploaded_at} />
                  {version.has_pdf && s.submission ? (
                    <>
                      {' '}
                      ·{' '}
                      <a href={pdfHref(s.submission, version.number)}>
                        {t('Version ')}
                        {version.number} {t(' PDF')}
                      </a>
                    </>
                  ) : null}
                </li>
              ))}
            </ol>
          </section>
        ) : null}
        <Citation paper={s} locator={{ version: paper.publication?.version ?? latest?.number }} />
        <Bibliography paper={paper} />
        <ProblemLinks links={paper.problems ?? []} />
        {paper.origin ? <ManuscriptSources origin={paper.origin} /> : null}
        {claims.length ? (
          <PaperResults paper={paper} />
        ) : (
          <p className="muted">{t('The author keeps the mathematical details private.')}</p>
        )}
        {claims.length ? (
          <details className="history">
            <summary>{t('Corrections')}</summary>
            <ReportError record={s.record} subject={{ kind: 'review' }} />
            <ErrorReports record={s.record} />
          </details>
        ) : null}
      </article>
    </MacrosProvider>
  );
}
