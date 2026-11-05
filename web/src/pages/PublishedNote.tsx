import { t } from '../i18n/locale';
import { AuthorLine } from '../components/AuthorLine';
import { PaperExplanation } from '../components/PaperExplanation';
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
        <a href={note.source.url}>{t('Source files')}</a>
        {note.lean ? <ExternalLink href={note.lean.url}>{t('Lean source')}</ExternalLink> : null}
        <span>{t('Version {version}', { version: note.revision })}</span>
        <DateText iso={note.published_on} />
      </p>
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
        {note.lean ? <ExternalLink href={note.lean.url}>{t('Lean source')}</ExternalLink> : null}
        <Link to={{ kind: 'research_graph', focus: `statement:${note.id}:${note.revision}:s1` }}>
          {t('Explore mathematical results')}
        </Link>
      </nav>
      <section>
        <h2>{t('Abstract')}</h2>
        <LatexText source={note.abstract} />
      </section>
      <PaperExplanation text={note.plain_language} />
      {note.problems.length ? (
        <details className="paper-details">
          <summary>{t('Sources and related questions')}</summary>
          <ul className="paper-questions">
            {note.problems.map((problem, index) => {
              const href = sourceHref(problem.source);
              return (
                <li key={index}>
                  <InlineLatex source={problem.name} />
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
      <details className="paper-details paper-citation">
        <summary>{t('Citation and versions')}</summary>
        <p>{projectCitation(note)}</p>
        {versions.length ? (
          <ul>
            {versions.map((item) => (
              <li key={item.revision}>
                <Link to={{ kind: 'paper', record: id, version: item.revision }}>
                  {t('Version {version}', { version: item.revision })}
                </Link>
                {' · '}
                <a href={item.pdf.url}>PDF</a>
                {' · '}
                <a href={item.source.url}>{t('Source files')}</a>
              </li>
            ))}
          </ul>
        ) : null}
      </details>
    </article>
  );
}
