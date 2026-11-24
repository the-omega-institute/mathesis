import { useEffect } from 'react';
import { t } from '../i18n/locale';
import { PaperResearch } from '../components/PaperResearch';
import { publishedLeanSource } from '../lib/publishedLean';
import { sharedLibraryFocus } from '../lib/sharedLibrary';
import { AuthorLine } from '../components/AuthorLine';
import { PaperMaterialLinks, PaperMaterials } from '../components/PaperMaterials';
import { InlineLatex, LatexText } from '../components/Markdown';
import { DateText, ExternalLink } from '../components/ui';
import { sourceHref, sourceLabel } from '../lib/links';
import { CitationMetadata } from '../components/PaperMetadata';
import { routePath, paperRoute } from '../routing/routes';
import {
  projectCitation,
  projectNotes,
  projectNoteVersions,
  type ProjectNote,
} from '../lib/projectNotes';
import { Link, useRouter } from '../routing/router';
import { NotFoundPage } from './NotFound';
import '../styles/published-note.css';

export function PublishedNoteEntry({ note }: { note: ProjectNote }) {
  return (
    <article className="paper-entry research-entry">
      <div className="research-entry-body">
        <h3>
          <Link to={paperRoute(note.id, note.archive?.doi)}>
            <InlineLatex source={note.title} />
          </Link>
        </h3>
        <AuthorLine authors={note.authors.map(({ name }) => ({ name }))} />
        <p className="entry-meta">
          <span>{t('Version {version}', { version: note.revision })}</span>
          {note.archive ? (
            <ExternalLink href={`https://doi.org/${note.archive.doi}`}>DOI</ExternalLink>
          ) : null}
          {note.lean ? <ExternalLink href={note.lean.url}>{t('Lean source')}</ExternalLink> : null}
        </p>
      </div>
      <div className="research-entry-access">
        <DateText iso={note.published_on} />
        <div className="research-entry-links">
          <a href={note.pdf.url}>{t('Read PDF')}</a>
          <a href={note.source.url}>{t('Source files')}</a>
        </div>
      </div>
    </article>
  );
}

export function PublishedNotePage({
  id,
  version,
  note: supplied,
}: {
  id: string;
  version?: string;
  note?: ProjectNote;
}) {
  const versions = supplied ? [supplied] : projectNoteVersions.filter((note) => note.id === id);
  const note = version
    ? versions.find((note) => note.revision === version)
    : (supplied ?? projectNotes.find((note) => note.id === id));
  const { route, navigate } = useRouter();
  const routeRecord = route.kind === 'paper' ? route.record : undefined;
  const routeVersion = route.kind === 'paper' ? route.version : undefined;
  const canonicalRecord = note ? paperRoute(note.id, note.archive?.doi).record : undefined;
  useEffect(() => {
    if (canonicalRecord?.startsWith('doi:') && routeRecord && routeRecord !== canonicalRecord)
      navigate(
        { kind: 'paper', record: canonicalRecord, version: routeVersion },
        { replace: true },
      );
  }, [canonicalRecord, routeRecord, routeVersion, navigate]);
  if (!note) return <NotFoundPage path={`/papers/${id}`} />;
  return (
    <article className="page published-note paper-reading">
      <CitationMetadata
        title={note.title}
        authors={note.authors}
        doi={note.archive?.doi}
        abstract={note.abstract}
        publicationDate={note.published_on}
        canonicalUrl={`${window.location.origin}${routePath(paperRoute(note.id, note.archive?.doi, note.archive?.doi ? undefined : note.revision))}`}
        pdfUrl={new URL(note.pdf.url, window.location.origin).href}
      />
      <Link to={{ kind: 'papers' }}>{t('Research archive')}</Link>
      <header className="paper-heading">
        <div>
          <h1>
            <InlineLatex source={note.title} />
          </h1>
          <AuthorLine authors={note.authors} />
          <p className="entry-meta">
            {t('Version {version}', { version: note.revision })} ·{' '}
            <DateText iso={note.published_on} />
          </p>
        </div>
      </header>
      <nav className="paper-access" aria-label={t('Read this work')}>
        <a href={note.pdf.url}>{t('Read PDF')}</a>
        <a href={note.source.url} download>
          {t('Download source')}
        </a>
        {note.lean ? <ExternalLink href={note.lean.url}>{t('Lean source')}</ExternalLink> : null}
        {sharedLibraryFocus(note.lean?.url) ? (
          <Link
            to={{
              kind: 'formalizations',
              focus: sharedLibraryFocus(note.lean?.url),
            }}
          >
            {t('View in dependency graph')}
          </Link>
        ) : null}
        {note.materials ? (
          <ExternalLink href={note.materials.repository_url}>{t('Repository')}</ExternalLink>
        ) : null}
      </nav>
      <nav className="paper-sections" aria-label={t('On this page')}>
        <a href="#further-work-h">{t('Continue this research')}</a>
        <PaperMaterialLinks materials={note.materials} text={note.plain_language} />
        {note.problems.length ? (
          <a
            href="#related-questions"
            onClick={() => document.getElementById('related-questions')?.setAttribute('open', '')}
          >
            {t('Sources and related questions')}
          </a>
        ) : null}
        <a
          href="#citation"
          onClick={() => document.getElementById('citation')?.setAttribute('open', '')}
        >
          {t('Citation and versions')}
        </a>
      </nav>
      <div className="paper-reading-layout">
        <div className="paper-content">
          <section aria-labelledby="abstract-h" className="paper-abstract">
            <h2 id="abstract-h">{t('Abstract')}</h2>
            <LatexText source={note.abstract} className="abstract" />
          </section>
          {note.problems.length ? (
            <details className="paper-details" id="related-questions" open>
              <summary>{t('Sources and related questions')}</summary>
              <ul className="paper-questions">
                {note.problems.map((problem, index) => {
                  const href = sourceHref(problem.source);
                  return (
                    <li key={index}>
                      {problem.id ? (
                        <Link to={{ kind: 'research_problem', id: problem.id }}>
                          <InlineLatex source={problem.name} />
                        </Link>
                      ) : (
                        <InlineLatex source={problem.name} />
                      )}
                      {href ? (
                        <span>
                          {' '}
                          · <ExternalLink href={href}>{sourceLabel(problem.source)}</ExternalLink>
                        </span>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            </details>
          ) : null}
          <PaperResearch
            work={note.id}
            version={Number(note.revision)}
            origin={
              new URL(
                routePath(
                  paperRoute(
                    note.id,
                    note.archive?.doi,
                    note.archive?.doi ? undefined : note.revision,
                  ),
                ),
                window.location.origin,
              ).href
            }
            materials={note.materials}
            leanSource={publishedLeanSource(note)}
          />
          <PaperMaterials materials={note.materials} text={note.plain_language} />
        </div>
        <aside className="paper-sidebar" aria-label={t('Publication details')}>
          <section>
            <h2>{t('Publication details')}</h2>
            <dl className="paper-metadata">
              {note.archive ? (
                <>
                  <dt>DOI</dt>
                  <dd>
                    <ExternalLink href={`https://doi.org/${note.archive.doi}`}>
                      {note.archive.doi}
                    </ExternalLink>
                  </dd>
                </>
              ) : null}
              <dt>{t('License')}</dt>
              <dd>{note.license}</dd>
              {note.msc?.length ? (
                <>
                  <dt>{t('Subjects')}</dt>
                  <dd>
                    {note.msc.map((code, index) => (
                      <span key={code}>
                        {index ? '; ' : ''}
                        <Link to={{ kind: 'papers', msc: code }}>MSC {code}</Link>
                      </span>
                    ))}
                  </dd>
                </>
              ) : null}
            </dl>
          </section>
          <details id="citation" className="paper-citation citation">
            <summary>{t('Cite this paper')}</summary>
            <p>{projectCitation(note)}</p>
          </details>
          {versions.length > 1 || note.previous_version ? (
            <section className="paper-versions" aria-labelledby="public-versions-h">
              <h2 id="public-versions-h">{t('Version history')}</h2>
              <ul className="version-list">
                {versions.map((item) => (
                  <li key={item.revision}>
                    {item.revision === note.revision ? (
                      <span aria-current="page">
                        {t('Version {version}', { version: item.revision })}
                      </span>
                    ) : (
                      <Link
                        to={paperRoute(
                          id,
                          item.archive?.doi,
                          item.archive?.doi ? undefined : item.revision,
                        )}
                      >
                        {t('Version {version}', { version: item.revision })}
                      </Link>
                    )}
                    {' · '}
                    <DateText iso={item.published_on} />
                    {item.revision !== note.revision ? (
                      <div className="paper-version-files">
                        <a href={item.pdf.url}>PDF</a>
                        <a href={item.source.url}>{t('Source files')}</a>
                      </div>
                    ) : null}
                  </li>
                ))}
                {note.previous_version &&
                !versions.some((item) => item.revision === note.previous_version) ? (
                  <li>
                    <Link to={paperRoute(id, undefined, note.previous_version)}>
                      {t('Version {version}', { version: note.previous_version })}
                    </Link>
                  </li>
                ) : null}
              </ul>
            </section>
          ) : null}
        </aside>
      </div>
    </article>
  );
}
