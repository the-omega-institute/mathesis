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
      <div className="research-entry-body">
        <h3>
          <Link to={{ kind: 'paper', record: note.id }}>
            <InlineLatex source={note.title} />
          </Link>
        </h3>
        <AuthorLine authors={note.authors.map(({ name }) => ({ name }))} />
        <p className="entry-meta">
          <span>{t('Version {version}', { version: note.revision })}</span>
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
  if (!note) return <NotFoundPage path={`/papers/${id}`} />;
  return (
    <article className="page published-note paper-reading">
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
        <Link to={{ kind: 'research_graph', focus: `statement:${note.id}:${note.revision}:s1` }}>
          {t('Explore mathematical results')}
        </Link>
      </nav>
      <nav className="paper-sections" aria-label={t('On this page')}>
        <a href="#abstract-h">{t('Abstract')}</a>
        {note.plain_language?.en.trim() && note.plain_language.zh.trim() ? (
          <a href="#plain-language">{t('In plain language')}</a>
        ) : null}
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
          <PaperExplanation text={note.plain_language} />
          {note.problems.length ? (
            <details className="paper-details" id="related-questions">
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
        </div>
        <aside className="paper-sidebar" aria-label={t('Publication details')}>
          <section>
            <h2>{t('Publication details')}</h2>
            <dl className="paper-metadata">
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
          {versions.length ? (
            <section className="paper-versions" aria-labelledby="public-versions-h">
              <h2 id="public-versions-h">{t('Version history')}</h2>
              <ul className="version-list">
                {versions.map((item) => (
                  <li key={item.revision}>
                    <Link to={{ kind: 'paper', record: id, version: item.revision }}>
                      {t('Version {version}', { version: item.revision })}
                    </Link>
                    {' · '}
                    <DateText iso={item.published_on} />
                    <div className="paper-version-files">
                      <a href={item.pdf.url}>PDF</a>
                      <a href={item.source.url}>{t('Source files')}</a>
                    </div>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </aside>
      </div>
    </article>
  );
}
