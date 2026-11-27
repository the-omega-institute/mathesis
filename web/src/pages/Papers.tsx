import { t } from '../i18n/locale';
import { PaperMaterialLinks, PaperMaterials } from '../components/PaperMaterials';
import {
  collectedPaperNote,
  findProjectNotes,
  mergeCollectedNotes,
  projectNoteVersions,
} from '../lib/projectNotes';
import { researchEntries } from '../lib/researchEntries';
import { findSubject, subjectGroups, type Subject } from '../lib/subjects';
import { listDirectoryNotes } from '../lib/paperDirectory';
import { resolvePaperDoi } from '../lib/paperDoi';
import { paperRoute } from '../routing/routes';
import { NotFoundPage } from './NotFound';
import { PaperDirectoryEntry } from '../components/PaperDirectoryEntry';
import { PublishedNotePage } from './PublishedNote';
import { ProblemLinks } from './ResearchProblems';
import { Bibliography } from '../components/Bibliography';
import { InlineLatex } from '../components/Markdown';
import { ManuscriptSources } from '../components/ManuscriptSources';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { useCallback, useEffect, useState } from 'react';
import { pdfHref, type PaperQuery } from '../api/client';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { usePaged } from '../api/usePaged';
import type { PaperSummary, PublicPaper } from '../api/types';
import { AuthorLine } from '../components/AuthorLine';
import { Citation } from '../components/Citation';
import { PaperMetadata } from '../components/PaperMetadata';
import { PaperResearch } from '../components/PaperResearch';
import { routePath } from '../routing/routes';
import { PaperResults } from '../components/PaperResults';
import { LatexText, MacrosProvider } from '../components/Markdown';
import { Pager } from '../components/Pager';
import { Async, DateText, ExternalLink, MscList } from '../components/ui';
import { isOpenKind, SUBMISSION_KIND_LABELS } from '../lib/labels';
import { safeHttpUrl, sourceHref } from '../lib/links';
import { Link, useRouter } from '../routing/router';

export function PaperEntry({ paper }: { paper: PaperSummary }) {
  return (
    <article className="paper-entry research-entry">
      <div className="research-entry-body">
        <h3>
          <Link to={{ kind: 'paper', record: paper.record }}>
            <InlineLatex source={paper.title} />
          </Link>
        </h3>
        <AuthorLine authors={paper.authors.map(({ name }) => ({ name }))} />
        <div className="entry-meta">
          <span>{paper.record}</span>
          <span>{t(SUBMISSION_KIND_LABELS[paper.kind])}</span>
          <MscList codes={paper.msc ?? []} />
        </div>
      </div>
      <div className="research-entry-access">
        {paper.accepted_at ? <DateText iso={paper.accepted_at} /> : null}
        <Link to={{ kind: 'paper', record: paper.record }}>{t('View paper')}</Link>
      </div>
    </article>
  );
}

export function PapersPage({
  initialQuery = '',
  initialFilters = {},
  subject,
}: {
  initialQuery?: string;
  initialFilters?: Pick<PaperQuery, 'author' | 'msc' | 'kind'>;
  subject?: Subject;
} = {}) {
  const api = useApi();
  const { navigate } = useRouter();
  const [query, setQuery] = useState<PaperQuery>({
    ...initialFilters,
    ...(initialFilters.kind ? {} : { exclude_kind: 'methodology' as const }),
    ...(initialQuery ? { q: initialQuery } : {}),
  });
  const [draft, setDraft] = useState<PaperQuery>(query);
  const fetchPage = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.listPapers({ ...query, before, limit: 25 }, { signal }),
    [api, query],
  );
  const paged = usePaged(fetchPage);
  const collected = useAsync(
    useCallback((signal: AbortSignal) => listDirectoryNotes(api, query, signal), [api, query]),
  );
  // A locally published manuscript appears once, on the first archive page.
  // Following the API cursor must not repeat it on every subsequent page.
  const notes =
    paged.page === 1
      ? findProjectNotes(query, mergeCollectedNotes(collected.value?.items ?? []))
      : [];
  const entries = researchEntries(paged.value?.items ?? [], notes);
  return (
    <div className="page research-archive">
      {subject ? (
        <nav className="subject-navigation" aria-label={t('Research navigation')}>
          <Link to={{ kind: 'papers' }}>{t('Research archive')}</Link>
          <Link to={{ kind: 'subjects' }}>{t('All subjects')}</Link>
        </nav>
      ) : null}
      <header className="page-head">
        <div>
          <h1>{t(subject?.title ?? 'Papers')}</h1>
          {subject ? <p className="muted">MSC {subject.code}</p> : null}
        </div>
      </header>
      <nav className="archive-subjects" aria-label={t('Research tracks')}>
        <Link
          to={{ kind: 'papers' }}
          aria-current={query.kind !== 'methodology' && !query.msc ? 'page' : undefined}
        >
          {t('Mathematics')}
        </Link>
        <Link
          to={{ kind: 'papers', work: 'methodology' }}
          aria-current={query.kind === 'methodology' ? 'page' : undefined}
        >
          {t('Methodology')}
        </Link>
      </nav>
      <div className="archive-browse">
        <nav className="archive-subjects" aria-label={t('Browse by subject')}>
          <Link
            to={{ kind: 'papers' }}
            aria-current={!query.msc && query.kind !== 'methodology' ? 'page' : undefined}
          >
            {t('All papers')}
          </Link>
          {['05', '11', '81', '15'].map((code) => (
            <Link
              key={code}
              to={{ kind: 'subject', msc: code }}
              aria-current={query.msc === code ? 'page' : undefined}
            >
              {t(findSubject(code)!.title)}
            </Link>
          ))}
        </nav>
        <label className="archive-subject-picker">
          <span className="visually-hidden">{t('Choose a subject')}</span>
          <select
            value={query.msc?.slice(0, 2) ?? ''}
            onChange={(event) => {
              const msc = event.target.value;
              navigate(msc ? { kind: 'subject', msc } : { kind: 'papers' });
            }}
          >
            <option value="">{t('All subjects')}</option>
            {subjectGroups.map((group) => (
              <optgroup key={group.title} label={t(group.title)}>
                {group.subjects.map((subject) => (
                  <option key={subject.code} value={subject.code}>
                    {t(subject.title)}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        </label>
      </div>
      {subject ? null : (
        <details
          className="archive-search-disclosure"
          open={
            Boolean(
              initialQuery || initialFilters.author || initialFilters.msc || initialFilters.kind,
            ) || undefined
          }
        >
          <summary>{t('Search and filters')}</summary>
          <form
            className="archive-search"
            role="search"
            aria-label={t('Search papers')}
            onSubmit={(event) => {
              event.preventDefault();
              setQuery({ ...draft, exclude_kind: draft.kind ? undefined : 'methodology' });
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
                  onChange={(e) =>
                    setDraft({ ...draft, kind: e.target.value as PaperQuery['kind'] })
                  }
                >
                  <option value="">{t('All research')}</option>
                  <option value="paper">{t('Papers')}</option>
                  <option value="note">{t('Short notes')}</option>
                  <option value="methodology">{t('Methodology')}</option>
                  <option value="conjecture">{t('Conjectures')}</option>
                </select>
              </label>
              <button
                className="button button-quiet"
                type="button"
                onClick={() => {
                  setDraft({ exclude_kind: 'methodology' });
                  setQuery({ exclude_kind: 'methodology' });
                  navigate({ kind: 'papers' });
                }}
              >
                {t('Clear filters')}
              </button>
            </div>
          </form>
        </details>
      )}
      {entries.length ? (
        <ul className="entry-list research-archive-list" role="list">
          {entries.map((entry) => (
            <li
              key={entry.kind === 'note' ? `note:${entry.note.id}` : `paper:${entry.paper.record}`}
            >
              <PaperDirectoryEntry entry={entry} />
            </li>
          ))}
        </ul>
      ) : null}
      <Async state={paged.state} onRetry={paged.reload}>
        {(listing) =>
          listing.items.length === 0 && notes.length === 0 && collected.state.status === 'ok' ? (
            <p className="muted">
              {t(
                subject
                  ? 'No papers have been listed in this subject yet.'
                  : 'No papers match this view. Try a broader search or clear the filters.',
              )}
            </p>
          ) : null
        }
      </Async>
      <Async state={collected.state} onRetry={collected.reload}>
        {() => null}
      </Async>
      <Pager paged={paged} />
    </div>
  );
}

export function PaperPage({ record, version }: { record: string; version?: string }) {
  if (record.startsWith('doi:')) return <DoiPaperPage doi={record.slice(4)} version={version} />;
  return record.startsWith('WP-') ? (
    <ReviewedPaperPage record={record} />
  ) : (
    <CollectedPaperPage record={record} version={version} />
  );
}

function DoiPaperPage({ doi, version }: { doi: string; version?: string }) {
  const api = useApi();
  const paper = useAsync(
    useCallback((signal: AbortSignal) => resolvePaperDoi(api, doi, signal), [api, doi]),
  );
  return (
    <Async state={paper.state} onRetry={paper.reload}>
      {(resolved) =>
        resolved && (!version || version === resolved.version) ? (
          <PaperPage record={resolved.record} version={resolved.version} />
        ) : (
          <NotFoundPage path={'/papers/doi/' + doi} />
        )
      }
    </Async>
  );
}

function CollectedPaperPage({ record, version }: { record: string; version?: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.getCollectedPaper(record, version, { signal }),
    [api, record, version],
  );
  const paper = useAsync(load);
  const fallback = projectNoteVersions.find(
    (note) => note.id === record && (!version || note.revision === version),
  );
  if (paper.state.status !== 'ok' && fallback) {
    return <PublishedNotePage id={record} version={version} />;
  }
  return (
    <div className="page">
      <Async state={paper.state} onRetry={paper.reload}>
        {(p) => {
          const note = collectedPaperNote(p);
          return <PublishedNotePage id={note.id} version={note.revision} note={note} />;
        }}
      </Async>
    </div>
  );
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
  const { route, navigate } = useRouter();
  const routeRecord = route.kind === 'paper' ? route.record : undefined;
  const canonicalRecord = paperRoute(
    paper.summary.record,
    paper.archive?.doi ?? paper.summary.doi,
  ).record;
  useEffect(() => {
    if (routeRecord && routeRecord !== canonicalRecord)
      navigate({ kind: 'paper', record: canonicalRecord }, { replace: true });
  }, [routeRecord, navigate, canonicalRecord]);
  const s = paper.summary;
  const versions = paper.versions ?? [];
  const latest = versions.at(-1);
  const claims = paper.claims ?? [];
  const archiveDoi = paper.archive?.doi ?? undefined;
  const repo = paper.formalization_repository ? safeHttpUrl(paper.formalization_repository) : null;
  const arxiv = /^10\.48550\/arxiv\.(.+)$/i.exec(s.doi ?? '')?.[1];
  const doiHref = s.doi ? sourceHref({ kind: 'doi', locator: s.doi }) : null;
  return (
    <MacrosProvider macros={paper.macros ?? {}}>
      <article className="paper academic-paper paper-reading">
        <PaperMetadata paper={paper} />
        <p className="entry-meta">
          <Link to={{ kind: 'papers' }}>{t('Research archive')}</Link> · {s.record} ·{' '}
          {t(SUBMISSION_KIND_LABELS[s.kind])}
        </p>
        <header className="paper-heading">
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
            {latest ? ` · ${t('Version {version}', { version: latest.number })}` : null}
          </p>
          <nav className="paper-access" aria-label={t('Paper files and citation')}>
            {latest?.has_pdf && s.submission ? (
              <a href={pdfHref(s.submission, latest.number)}>{t('View PDF')}</a>
            ) : null}
            {paper.archive ? <a href={paper.archive.source.url}>{t('Download source')}</a> : null}
            {paper.materials ? (
              <ExternalLink href={paper.materials.repository_url}>{t('Repository')}</ExternalLink>
            ) : null}
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
        <nav className="paper-sections" aria-label={t('On this page')}>
          {claims.length ? <a href="#further-work-h">{t('Continue this research')}</a> : null}
          <PaperMaterialLinks materials={paper.materials} text={paper.plain_language} />
          {claims.some((claim) => !isOpenKind(claim.kind)) ? (
            <a href="#results-h">{t('Results')}</a>
          ) : null}
          {paper.publication ? <a href="#references-h">{t('References and connections')}</a> : null}
          <a
            href="#citation"
            onClick={() => document.getElementById('citation')?.setAttribute('open', '')}
          >
            {t('Citation and versions')}
          </a>
        </nav>
        <div className="paper-reading-layout">
          <div className="paper-content">
            {s.abstract_text ? (
              <section aria-labelledby="abstract-h" className="paper-abstract">
                <h2 id="abstract-h">{t('Abstract')}</h2>
                <LatexText source={s.abstract_text} className="abstract" />
              </section>
            ) : null}
            <ProblemLinks links={paper.problems ?? []} />
            {claims.length ? (
              <PaperResearch
                macros={paper.macros}
                work={s.record}
                version={paper.publication?.version ?? latest?.number ?? 1}
                origin={
                  new URL(routePath(paperRoute(s.record, archiveDoi)), window.location.origin).href
                }
                materials={paper.materials}
                fallback={<PaperResults paper={paper} />}
              />
            ) : (
              <p className="muted">{t('The author keeps the mathematical details private.')}</p>
            )}
            <PaperMaterials materials={paper.materials} text={paper.plain_language} />
            {claims.length ? (
              <details className="history">
                <summary>{t('Corrections')}</summary>
                <ReportError record={s.record} subject={{ kind: 'review' }} />
                <ErrorReports record={s.record} />
              </details>
            ) : null}
            <Bibliography paper={paper} />
            {paper.origin ? <ManuscriptSources origin={paper.origin} /> : null}
          </div>
          <aside className="paper-sidebar" aria-label={t('Publication details')}>
            {s.msc?.length || s.doi || paper.archive ? (
              <section>
                <h2>{t('Publication details')}</h2>
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
                            href={
                              arxiv ? `https://arxiv.org/abs/${encodeURIComponent(arxiv)}` : doiHref
                            }
                          >
                            {arxiv ? `arXiv:${arxiv}` : s.doi}
                          </ExternalLink>
                        ) : (
                          s.doi
                        )}
                      </dd>
                    </>
                  ) : null}
                  {archiveDoi ? (
                    <>
                      <dt>{t('Archive DOI')}</dt>
                      <dd>
                        <ExternalLink href={`https://doi.org/${archiveDoi}`}>
                          {archiveDoi}
                        </ExternalLink>
                      </dd>
                    </>
                  ) : null}
                  {paper.archive ? (
                    <>
                      <dt>{t('License')}</dt>
                      <dd>{paper.archive.licence}</dd>
                    </>
                  ) : null}
                </dl>
              </section>
            ) : null}
            <Citation
              paper={s}
              locator={{ version: paper.publication?.version ?? latest?.number }}
              archiveDoi={archiveDoi}
            />
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
                      {version.number !== latest?.number && version.has_pdf && s.submission ? (
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
          </aside>
        </div>
      </article>
    </MacrosProvider>
  );
}
