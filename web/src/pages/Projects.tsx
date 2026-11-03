import { t } from '../i18n/locale';
import { AuthorLine } from '../components/AuthorLine';
import { InlineLatex, LatexText } from '../components/Markdown';
import { ExternalLink } from '../components/ui';
import { sourceHref, sourceLabel } from '../lib/links';
import {
  projectCitation,
  projectNotes,
  projectNoteVersions,
  projectProblemCount,
  type ProjectNote,
} from '../lib/projectNotes';
import { Link } from '../routing/router';
import { NotFoundPage } from './NotFound';

export function ProjectNoteEntry({
  note,
  compact = false,
}: {
  note: ProjectNote;
  compact?: boolean;
}) {
  return (
    <article className="project-note-entry">
      <h3>
        <Link to={{ kind: 'project_note', id: note.id }}>
          <InlineLatex source={note.title} />
        </Link>
      </h3>
      <AuthorLine authors={note.authors} />
      {compact ? null : <LatexText source={note.summary} />}
      <p className="entry-meta">
        <a href={note.pdf.url}>{t('Read PDF')}</a>
        <ExternalLink href={note.lean.url}>{t('Lean source')}</ExternalLink>
        <span>{t('Version {version}', { version: note.revision })}</span>
      </p>
    </article>
  );
}

export function ProjectsPage() {
  return (
    <div className="page project-collection">
      <header className="page-head">
        <div>
          <p className="eyebrow">{t('Research collections')}</p>
          <h1>Trureturing</h1>
          <p className="lede">
            {t(
              'From open mathematical questions to readable proofs and reusable Lean mathematics.',
            )}
          </p>
        </div>
      </header>
      <p className="project-introduction">
        {t(
          'Read the original questions, follow the arguments, and build on the formalized results. These notes retain their authors, original sources and review from the Trureturing research workflow.',
        )}
      </p>
      <p className="entry-meta">
        <span>{t('{count} notes', { count: projectNotes.length })}</span>
        <span>{t('{count} associated problems', { count: projectProblemCount })}</span>
        <ExternalLink href="https://github.com/the-omega-institute/trureturing">
          {t('Project source')}
        </ExternalLink>
      </p>
      <section aria-labelledby="project-notes-heading">
        <h2 id="project-notes-heading">{t('Papers and notes')}</h2>
        <ul className="project-notes-list">
          {projectNotes.map((note) => (
            <li key={note.id}>
              <ProjectNoteEntry note={note} />
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

export function ProjectNotePage({ id, version }: { id: string; version?: string }) {
  const versions = projectNoteVersions.filter((note) => note.id === id);
  const note = version
    ? versions.find((note) => note.revision === version)
    : projectNotes.find((note) => note.id === id);
  if (!note) return <NotFoundPage path={`/projects/trureturing/notes/${id}`} />;
  return (
    <article className="page project-note">
      <Link to={{ kind: 'projects' }}>{t('Trureturing collection')}</Link>
      <header className="page-head">
        <div>
          <h1>
            <InlineLatex source={note.title} />
          </h1>
          <AuthorLine authors={note.authors} />
          <p className="entry-meta">
            {t('Version {version}', { version: note.revision })} · {note.license}
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
      <p className="small muted">
        {t('Included from the Trureturing research workflow, with its upstream manuscript review.')}
      </p>
      <section>
        <h2>{t('Abstract')}</h2>
        <LatexText source={note.abstract} />
      </section>
      {note.problems.length ? (
        <section>
          <h2>{t('Original questions and sources')}</h2>
          <ul className="project-questions">
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
        <p>
          {t(
            'The note describes which results its Lean sources cover. Open the pinned source to inspect the exact statements and their mathematical prerequisites.',
          )}
        </p>
        <p>
          <ExternalLink href={note.lean.url}>{t('Read the Lean source')}</ExternalLink>
        </p>
        <p>
          {t(
            'A source link records existing formalization work. Mathesis verification and contribution credit are recorded separately when a proof is checked here.',
          )}
        </p>
        <p>
          <Link to={{ kind: 'formalizations' }}>{t('Contribute a formalization')}</Link>
        </p>
      </section>
      <details className="project-citation">
        <summary>{t('Cite this note')}</summary>
        <p>{projectCitation(note)}</p>
      </details>
      <section>
        <h2>{t('Versions')}</h2>
        <ul>
          {versions.map((item) => (
            <li key={item.revision}>
              <Link to={{ kind: 'project_note', id, version: item.revision }}>
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
