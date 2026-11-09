import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { MacrosProvider } from '../components/Markdown';
import { t } from '../i18n/locale';
import { AuthorLine } from '../components/AuthorLine';
import { PaperExplanation } from '../components/PaperExplanation';
import { InlineLatex, LatexText } from '../components/Markdown';
import { DateText, ExternalLink } from '../components/ui';
import { sourceHref, sourceLabel } from '../lib/links';
import { CitationMetadata } from '../components/PaperMetadata';
import { routePath } from '../routing/routes';
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
  if (!note) return <NotFoundPage path={`/papers/${id}`} />;
  return (
    <article className="page published-note paper-reading">
      <CitationMetadata
        title={note.title}
        authors={note.authors}
        doi={note.archive?.doi}
        abstract={note.abstract}
        publicationDate={note.published_on}
        canonicalUrl={`${window.location.origin}${routePath({ kind: 'paper', record: note.id, version: note.revision })}`}
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
          <PublishedFormalizations work={note.id} version={note.revision} />
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

function PublishedFormalizations({ work, version }: { work: string; version: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.collectedKnowledge(work, version, { signal }),
    [api, work, version],
  );
  const knowledge = useAsync(load);
  const rows = knowledge.value?.formalizations ?? [];
  if (!rows.length) return null;
  return (
    <section aria-labelledby="formalization-h">
      <h2 id="formalization-h">{t('Formalization')}</h2>
      <p>
        <Link
          to={{ kind: 'formalizations', focus: `statement:${work}:${version}:${rows[0]!.claim}` }}
        >
          {t('Explore mathematical results')}
        </Link>
      </p>
      <MacrosProvider macros={knowledge.value?.macros ?? {}}>
        <ul className="entry-list compact-formalizations">
          {rows.slice(0, 3).map((row) => {
            const claim = knowledge.value?.statements.find((c) => c.id === row.claim);
            if (!claim) return null;
            return (
              <li key={row.claim}>
                <Link to={{ kind: 'result', record: work, claim: row.claim, version }}>
                  <InlineLatex source={claim.title ?? claim.name} />
                </Link>
                {' · '}
                {t(
                  row.state === 'verified'
                    ? 'Verified in Lean'
                    : row.state === 'disproved'
                      ? 'Disproved in Lean'
                      : row.state === 'open'
                        ? 'Open for formalization'
                        : row.state === 'needs_attention'
                          ? 'Further proof work is needed'
                          : 'Awaiting a formal proof',
                )}
              </li>
            );
          })}
        </ul>
        {rows.length > 3 ? (
          <details className="history">
            <summary>{t('All formalization results ({count})', { count: rows.length })}</summary>
            <ul className="entry-list compact-formalizations">
              {rows.slice(3).map((row) => {
                const claim = knowledge.value?.statements.find((c) => c.id === row.claim);
                return claim ? (
                  <li key={row.claim}>
                    <Link to={{ kind: 'result', record: work, claim: row.claim, version }}>
                      <InlineLatex source={claim.title ?? claim.name} />
                    </Link>{' '}
                    ·{' '}
                    {t(
                      row.state === 'verified'
                        ? 'Verified in Lean'
                        : row.state === 'disproved'
                          ? 'Disproved in Lean'
                          : 'Awaiting a formal proof',
                    )}
                  </li>
                ) : null;
              })}
            </ul>
          </details>
        ) : null}
      </MacrosProvider>
    </section>
  );
}
