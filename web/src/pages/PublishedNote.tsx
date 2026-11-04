import { t } from '../i18n/locale';
import { AuthorLine } from '../components/AuthorLine';
import { InlineLatex, LatexText } from '../components/Markdown';
import { DateText, ExternalLink } from '../components/ui';
import { sourceHref, sourceLabel } from '../lib/links';
import {
  projectCitation,
  projectNotes,
  projectNoteVersions,
  type ProjectNote,
} from '../lib/projectNotes';
import { Link } from '../routing/router';
import { NotFoundPage } from './NotFound';
import '../styles/published-note.css';

export function PublishedNoteEntry({ note }: { note: ProjectNote }) {
  return (
    <article className="paper-entry research-entry">
      <h3>
        <Link to={{ kind: 'paper', record: note.id }}>
          <InlineLatex source={note.title} />
        </Link>
      </h3>
      <AuthorLine authors={note.authors.map(({ name }) => ({ name }))} />
      <p className="entry-meta research-entry-links">
        <a href={note.pdf.url}>{t('Read PDF')}</a>
        <a href={note.source.url}>{t('Source')}</a>
        <ExternalLink href={note.lean.url}>{t('Lean source')}</ExternalLink>
        <span>{t('Version {version}', { version: note.revision })}</span>
        <DateText iso={note.published_on} />
      </p>
    </article>
  );
}

export function PublishedNotePage({ id, version }: { id: string; version?: string }) {
  const versions = projectNoteVersions.filter((note) => note.id === id);
  const note = version
    ? versions.find((note) => note.revision === version)
    : projectNotes.find((note) => note.id === id);
  if (!note) return <NotFoundPage path={`/papers/${id}`} />;
  return (
    <article className="page published-note">
      <Link to={{ kind: 'papers' }}>{t('Research archive')}</Link>
      <header className="page-head">
        <div>
          <h1>
            <InlineLatex source={note.title} />
          </h1>
          <AuthorLine authors={note.authors} />
          <p className="entry-meta">
            {t('Version {version}', { version: note.revision })} ·{' '}
            <DateText iso={note.published_on} /> · {note.license}
          </p>
        </div>
      </header>
      <nav className="paper-access" aria-label={t('Read this work')}>
        <a href={note.pdf.url}>{t('Read PDF')}</a>
        <a href={note.source.url} download>
          {t('Download source')}
        </a>
        <ExternalLink href={note.lean.url}>{t('Lean source')}</ExternalLink>
      </nav>
      <section>
        <h2>{t('Abstract')}</h2>
        <LatexText source={note.abstract} />
      </section>
      {note.problems.length ? (
        <section>
          <h2>{t('Original questions and sources')}</h2>
          <ul className="paper-questions">
            {note.problems.map((problem, index) => {
              const href = sourceHref(problem.source);
              return (
                <li key={index}>
                  <InlineLatex source={problem.name} />
                  {href ? (
                    <p>
                      <ExternalLink href={href}>{sourceLabel(problem.source)}</ExternalLink>
                    </p>
                  ) : null}
                </li>
              );
            })}
          </ul>
        </section>
      ) : null}
      <section>
        <h2>{t('Formalization and further contributions')}</h2>
        <p>{t('See the Lean source for the formalized statements and their assumptions.')}</p>
        <p>
          <ExternalLink href={note.lean.url}>{t('Read the Lean source')}</ExternalLink>
        </p>
        <p>
          <Link to={{ kind: 'formalizations' }}>{t('Contribute a formalization')}</Link>
        </p>
      </section>
      <details className="paper-citation">
        <summary>{t('Cite this note')}</summary>
        <p>{projectCitation(note)}</p>
      </details>
      <section>
        <h2>{t('Versions')}</h2>
        <ul>
          {versions.map((item) => (
            <li key={item.revision}>
              <Link to={{ kind: 'paper', record: id, version: item.revision }}>
                {t('Version {version}', { version: item.revision })}
              </Link>
              {' · '}
              <a href={item.pdf.url}>PDF</a>
              {' · '}
              <a href={item.source.url}>{t('Source')}</a>
            </li>
          ))}
        </ul>
      </section>
    </article>
  );
}
