import { t } from '../i18n/locale';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type {
  Author,
  ConjectureSummary,
  FormalizationOpportunity,
  PaperSummary,
} from '../api/types';
import { Async, Badge, DateText } from '../components/ui';
import { InlineLatex, LatexText } from '../components/Markdown';
import { Link } from '../routing/router';
import { SUBMISSION_KIND_LABELS } from '../lib/labels';

type AuthorWorks = {
  profile: Author;
  papers: PaperSummary[];
  conjectures: ConjectureSummary[];
  formalizations: FormalizationOpportunity[];
};

function hasAuthor(authors: readonly Author[] | undefined, name: string): boolean {
  const wanted = name.trim().toLowerCase();
  return Boolean(authors?.some((author) => author.name.trim().toLowerCase() === wanted));
}

export function AuthorPage({ name }: { name: string }) {
  const api = useApi();
  const load = useCallback(
    async (signal: AbortSignal): Promise<AuthorWorks> => {
      const [papers, open, solved, disproved, formalizations] = await Promise.all([
        api.listPapers({ author: name, limit: 100 }, { signal }),
        api.listConjectures({ status: 'open', limit: 100 }, { signal }),
        api.listConjectures({ status: 'solved', limit: 100 }, { signal }),
        api.listConjectures({ status: 'disproved', limit: 100 }, { signal }),
        api.formalizationOpportunities({ q: name, limit: 100 }, { signal }),
      ]);
      const conjectures = [...open.items, ...solved.items, ...disproved.items].filter((item) =>
        hasAuthor(item.authors, name),
      );
      const paperAuthor = papers.items
        .find((paper) => hasAuthor(paper.authors, name))
        ?.authors.find((author) => author.name.trim().toLowerCase() === name.trim().toLowerCase());
      const conjectureAuthor = conjectures
        .find((item) => hasAuthor(item.authors, name))
        ?.authors?.find((author) => author.name.trim().toLowerCase() === name.trim().toLowerCase());
      const opportunityAuthor = formalizations.items
        .find((item) => hasAuthor(item.authors, name))
        ?.authors.find((author) => author.name.trim().toLowerCase() === name.trim().toLowerCase());
      return {
        profile: paperAuthor ?? conjectureAuthor ?? opportunityAuthor ?? { name },
        papers: papers.items,
        conjectures,
        formalizations: formalizations.items.filter((item) => hasAuthor(item.authors, name)),
      };
    },
    [api, name],
  );
  const works = useAsync(load);
  return (
    <div className="page author-page">
      <Async state={works.state} onRetry={works.reload}>
        {(value) => (
          <>
            <header className="page-head author-head">
              <div>
                <p className="eyebrow">{t('Author')}</p>
                <h1>{value.profile.name}</h1>
                {value.profile.affiliation ? (
                  <p className="lede">{value.profile.affiliation}</p>
                ) : null}
                <AuthorLinks author={value.profile} />
              </div>
            </header>

            <p className="author-counts" aria-label={t('Author contribution counts')}>
              <strong>{value.papers.length}</strong> {t(' papers and notes ·')}{' '}
              <strong>{value.conjectures.length}</strong> {t(' conjectures ·')}{' '}
              <strong>{value.formalizations.length}</strong> {t(' formalization targets')}
            </p>

            <section aria-labelledby="author-papers-h">
              <h2 id="author-papers-h">{t('Papers and notes')}</h2>
              {value.papers.length ? (
                <ul className="entry-list">
                  {value.papers.map((paper) => (
                    <li key={paper.record}>
                      <Link className="entry-title" to={{ kind: 'paper', record: paper.record }}>
                        <InlineLatex source={paper.title} />
                      </Link>
                      <p className="entry-meta">
                        <code>{paper.record}</code> · {t(SUBMISSION_KIND_LABELS[paper.kind])}
                        {paper.accepted_at ? (
                          <>
                            {' '}
                            · <DateText iso={paper.accepted_at} />
                          </>
                        ) : null}
                      </p>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="muted">{t('No accepted papers or notes found.')}</p>
              )}
            </section>

            <section aria-labelledby="author-conjectures-h">
              <h2 id="author-conjectures-h">{t('Conjectures')}</h2>
              {value.conjectures.length ? (
                <ul className="entry-list">
                  {value.conjectures.map((item) => (
                    <li key={`${item.record}:${item.claim}`}>
                      <Link
                        className="entry-title"
                        to={{ kind: 'conjecture', record: item.record, claim: item.claim }}
                      >
                        <InlineLatex source={item.title} />
                      </Link>
                      <p className="entry-meta">
                        <code>{item.record}</code> · {item.claim} ·{' '}
                        <Badge tone={item.status === 'open' ? 'muted' : 'good'}>
                          {item.status}
                        </Badge>
                      </p>
                      <LatexText source={item.statement} className="conjecture-one-line" />
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="muted">{t('No conjectures found.')}</p>
              )}
            </section>

            <section aria-labelledby="author-formalizations-h">
              <h2 id="author-formalizations-h">{t('Formalization')}</h2>
              {value.formalizations.length ? (
                <ul className="entry-list">
                  {value.formalizations.map((item) => (
                    <li key={`${item.record}:${item.claim}`}>
                      <Link
                        className="entry-title"
                        to={{ kind: 'statement', record: item.record, claim: item.claim }}
                      >
                        <InlineLatex source={item.title} />
                      </Link>
                      <p className="entry-meta">
                        <code>{item.record}</code> · {item.label} · {item.status}
                      </p>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="muted">{t('No formalization targets found.')}</p>
              )}
            </section>
          </>
        )}
      </Async>
    </div>
  );
}

function AuthorLinks({ author }: { author: Author }) {
  return (
    <p className="author-links">
      {author.orcid ? (
        <a href={`https://orcid.org/${author.orcid.replace(/^https?:\/\/orcid.org\//i, '')}`}>
          ORCID
        </a>
      ) : null}
      {author.google_scholar ? <a href={author.google_scholar}>{t('Google Scholar')}</a> : null}
      {author.website ? <a href={author.website}>{t('Personal website')}</a> : null}
    </p>
  );
}
