import { useCallback, useId, useState } from 'react';
import { useApi } from '../api/context';
import { usePaged } from '../api/usePaged';
import type { Listing, Author } from '../api/types';
import { t, useLocale } from '../i18n/locale';
import { Link } from '../routing/router';
import { InlineLatex } from './Markdown';
import { Async } from './ui';
import { Pager } from './Pager';
import {
  areaModules,
  directorySource,
  findTheoryArea,
  formalLibrary,
  libraryGroups,
  localize,
  moduleChildren,
  moduleLabel,
  moduleSource,
  researchAreas,
  theoryAreas,
  type TheoryArea,
} from '../lib/theoryLibrary';
import '../styles/theory-library.css';

export function TheoryLibrary({ focus }: { focus?: string }) {
  const locale = useLocale();
  const area = findTheoryArea(focus?.startsWith('theory:') ? focus.slice(7) : '');
  return area ? (
    <TheoryAreaReading key={area.id} area={area} />
  ) : (
    <>
      <p className="theory-library-intro">
        {t(
          'Start with a field of mathematics. Follow its established theory, read the research that extends it, and find work to contribute.',
        )}
      </p>
      <div className="theory-library-directory">
        {libraryGroups.map((group) => (
          <section key={group.title}>
            <h2>{t(group.title)}</h2>
            <ul>
              {group.areas.map((id) => {
                const item = findTheoryArea(id)!;
                return (
                  <li key={id}>
                    <h3>
                      <Link to={{ kind: 'research_graph', focus: `theory:${id}` }}>
                        {localize(item.title, locale)}
                      </Link>
                    </h3>
                    <p>{localize(item.description, locale)}</p>
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>
      <LibraryProvenance />
      <nav className="theory-library-related" aria-label={t('Research and participation')}>
        <Link to={{ kind: 'papers' }}>{t('Research papers')}</Link>
        <Link to={{ kind: 'conjectures' }}>{t('Open conjectures')}</Link>
        <Link to={{ kind: 'formalizations' }}>{t('Contribute a formal proof')}</Link>
      </nav>
    </>
  );
}

function TheoryAreaReading({ area }: { area: TheoryArea }) {
  const locale = useLocale();
  const modules = areaModules(area);
  return (
    <div className="theory-library-layout">
      <nav className="theory-library-outline" aria-label={t('Fields of mathematics')}>
        <Link className="theory-library-home" to={{ kind: 'research_graph' }}>
          {t('All fields')}
        </Link>
        <ul>
          {theoryAreas.map((item) => (
            <li key={item.id}>
              <Link
                to={{ kind: 'research_graph', focus: `theory:${item.id}` }}
                aria-current={item.id === area.id ? 'page' : undefined}
              >
                {localize(item.title, locale)}
              </Link>
            </li>
          ))}
        </ul>
      </nav>
      <article className="theory-area-reading">
        <header className="theory-area-heading">
          <h2>{localize(area.title, locale)}</h2>
          <p>{localize(area.description, locale)}</p>
        </header>
        <nav className="theory-reading-outline" aria-label={t('In this field')}>
          <a href="#established-theory">{t('Established theory')}</a>
          <a href="#research-in-theory">{t('Research in this field')}</a>
          <a href="#extend-theory">{t('Questions and formalization')}</a>
        </nav>
        <section id="established-theory" className="theory-reading-section">
          <h3>{t('Established theory')}</h3>
          <p className="theory-section-intro">
            {t(
              'Definitions and results in Mathlib, maintained by the Lean mathematical community.',
            )}
          </p>
          <ul className="theory-reading-topics">
            {area.topics.map((topic) => (
              <li key={topic.module}>
                <h4>
                  <a href={moduleSource(topic.module)}>{localize(topic.title, locale)}</a>
                </h4>
                <p>{localize(topic.description, locale)}</p>
              </li>
            ))}
          </ul>
          <details className="theory-full-directory">
            <summary>
              {t('Browse the full formal library in this field')}{' '}
              <span>({modules.length.toLocaleString(locale)})</span>
            </summary>
            <p className="small muted">
              {t(
                'Open a branch to reach its definitions and theorems. Source files retain their original authorship.',
              )}
            </p>
            <ul className="theory-source-tree">
              {area.roots.map((root) => (
                <LibraryBranch key={root} prefix={`Mathlib.${root}`} modules={modules} />
              ))}
            </ul>
          </details>
        </section>
        <section id="research-in-theory" className="theory-reading-section">
          <h3>{t('Research in this field')}</h3>
          <TheoryResearch area={area} />
        </section>
        <section id="extend-theory" className="theory-reading-section theory-participate">
          <h3>{t('Questions and formalization')}</h3>
          <p>
            {t(
              'A paper can add a theorem, settle a conjecture, or reveal a question worth pursuing. Formal proofs connect precise results to the existing library and preserve each contribution.',
            )}
          </p>
          <p className="theory-participation-links">
            <Link to={{ kind: 'conjectures' }}>{t('Explore open conjectures')}</Link>
            <Link to={{ kind: 'formalizations' }}>{t('Find a confirmed proof target')}</Link>
            <Link to={{ kind: 'submit', intake: 'lean' }}>{t('Contribute Lean work')}</Link>
          </p>
          <p className="small muted">
            {t(
              'A published Lean source and an independently checked proof are shown separately. Target-specific contributions open after the exact statement is confirmed.',
            )}
          </p>
        </section>
        <aside className="theory-related-fields">
          <h3>{t('Related fields')}</h3>
          <p>
            {area.related.map((id) => {
              const related = findTheoryArea(id)!;
              return (
                <Link key={id} to={{ kind: 'research_graph', focus: `theory:${id}` }}>
                  {localize(related.title, locale)}
                </Link>
              );
            })}
          </p>
        </aside>
        <LibraryProvenance />
      </article>
    </div>
  );
}

export function LibraryBranch({ prefix, modules }: { prefix: string; modules: readonly string[] }) {
  const [open, setOpen] = useState(false);
  const title = moduleLabel(prefix.split('.').at(-1)!);
  const children = moduleChildren(prefix, modules);
  const isFile = modules.includes(prefix);
  return (
    <li>
      {children.length ? (
        <details open={open} onToggle={(event) => setOpen(event.currentTarget.open)}>
          <summary>{title}</summary>
          {open ? (
            <>
              {isFile ? (
                <p className="theory-branch-source">
                  <a href={moduleSource(prefix)}>{t('Read the definitions and results')}</a>
                </p>
              ) : null}
              <ul>
                {children.map((child) => (
                  <LibraryBranch key={child} prefix={`${prefix}.${child}`} modules={modules} />
                ))}
              </ul>
            </>
          ) : null}
        </details>
      ) : (
        <a href={moduleSource(prefix)}>{title}</a>
      )}
    </li>
  );
}

type TheoryPaper = {
  id: string;
  version?: string;
  title: string;
  authors: Author[];
  areas: readonly string[];
  collected: boolean;
};

function TheoryResearch({ area }: { area: TheoryArea }) {
  const api = useApi();
  const [expanded, setExpanded] = useState(false);
  const id = useId();
  const fetch = useCallback(
    async (before: string | null, signal: AbortSignal): Promise<Listing<TheoryPaper>> => {
      // Both public services form one reading list. A completed service is not
      // restarted while the other still has pages; these cursors stay local.
      const cursors: { collected: string | null | false; papers: string | null | false } = before
        ? JSON.parse(before)
        : { collected: null, papers: null };
      const [collected, papers] = await Promise.all([
        cursors.collected === false
          ? Promise.resolve({ items: [], next_before: null })
          : api.listCollectedPapers({ limit: 100, before: cursors.collected }, { signal }),
        cursors.papers === false
          ? Promise.resolve({ items: [], next_before: null })
          : api.listPapers({ limit: 100, before: cursors.papers }, { signal }),
      ]);
      return {
        next_before:
          collected.next_before || papers.next_before
            ? JSON.stringify({
                collected: collected.next_before ?? false,
                papers: papers.next_before ?? false,
              })
            : null,
        items: [
          ...collected.items.map((paper) => ({
            id: paper.id,
            version: paper.revision,
            title: paper.title,
            authors: paper.authors,
            areas: researchAreas(paper.id, paper.msc),
            collected: true,
          })),
          ...papers.items
            .filter((p) => Boolean(p.accepted_at))
            .map((paper) => ({
              id: paper.record,
              version: undefined,
              title: paper.title,
              authors: paper.authors,
              areas: researchAreas(paper.record, paper.msc),
              collected: false,
            })),
        ],
      };
    },
    [api],
  );
  const page = usePaged(fetch);
  return (
    <Async state={page.state} onRetry={page.reload}>
      {(data) => {
        const matching = data.items.filter((paper) => paper.areas.includes(area.id));
        const visible = expanded ? matching : matching.slice(0, 4);
        return (
          <>
            {matching.length ? (
              <ul id={id} className="theory-research-list">
                {visible.map((paper) => (
                  <li key={paper.id}>
                    <h4>
                      <Link to={{ kind: 'paper', record: paper.id, version: paper.version }}>
                        <InlineLatex source={paper.title} />
                      </Link>
                    </h4>
                    <p>{paper.authors.map((author) => author.name).join('; ')}</p>
                    <Link
                      className="theory-result-link"
                      to={
                        paper.collected
                          ? {
                              kind: 'research_graph',
                              focus: `statement:${paper.id}:${paper.version}:s1`,
                            }
                          : { kind: 'paper', record: paper.id }
                      }
                    >
                      {t('Read the results and their connections')}
                    </Link>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">
                {t(
                  'No papers from this field appear on this page. The established theory above remains available.',
                )}
              </p>
            )}
            {matching.length > 4 ? (
              <button
                type="button"
                className="text-button"
                aria-expanded={expanded}
                aria-controls={id}
                onClick={() => setExpanded(!expanded)}
              >
                {expanded
                  ? t('Show fewer papers')
                  : t('Show all {count} papers on this page', { count: matching.length })}
              </button>
            ) : null}
            <Pager paged={page} labels={{ newer: '← Previous', older: 'Next →' }} />
            <p className="theory-research-navigation">
              <Link to={{ kind: 'submit' }}>{t('Submit research in this field')}</Link>
            </p>
            <p className="small muted">
              {t(
                'Papers are placed by subject. This does not assert that a paper uses every result listed above.',
              )}
            </p>
          </>
        );
      }}
    </Async>
  );
}

function LibraryProvenance() {
  const locale = useLocale();
  return (
    <details className="theory-library-provenance">
      <summary>{t('About this library')}</summary>
      <p>
        {t(
          'The formal foundation is Mathlib, the community-built Lean mathematical library. This directory covers all {count} Lean source files at the version used by our proof checker, including supporting tools. It is a guide to that library, not a claim to contain all human mathematics.',
          { count: formalLibrary.modules.length.toLocaleString(locale) },
        )}
      </p>
      <p>
        <a href={directorySource('Mathlib')}>{t('Mathlib source and contributors')}</a>
        {' · '}
        <a href={`${formalLibrary.repository}/blob/${formalLibrary.revision}/LICENSE`}>
          {t('Apache 2.0 license')}
        </a>
      </p>
      <p>
        {t(
          'The theory outline groups resources for reading. Manuscript references and independently checked proof dependencies retain their separate meanings.',
        )}
      </p>
    </details>
  );
}
