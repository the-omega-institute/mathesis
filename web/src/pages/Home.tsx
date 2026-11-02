import { t } from '../i18n/locale';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async, DateText } from '../components/ui';
import { AuthorLine } from '../components/AuthorLine';
import { InlineLatex } from '../components/Markdown';
import { ResearchCycle } from '../components/ResearchCycle';
import { Link } from '../routing/router';
import '../styles/home.css';

const SUBJECTS = [
  ['03', 'Logic and foundations'],
  ['05', 'Combinatorics'],
  ['11', 'Number theory'],
  ['14', 'Algebraic geometry'],
  ['20', 'Group theory'],
  ['35', 'Partial differential equations'],
  ['46', 'Functional analysis'],
  ['51', 'Geometry'],
  ['54', 'Topology'],
  ['60', 'Probability'],
  ['68', 'Computer science'],
  ['81', 'Quantum theory'],
] as const;

export function HomePage() {
  const api = useApi();
  const loadPapers = useCallback(
    (signal: AbortSignal) => api.listPapers({ limit: 3 }, { signal }),
    [api],
  );
  const papers = useAsync(loadPapers);
  return (
    <div className="page home">
      <header className="home-hero">
        <div className="home-introduction">
          <p className="eyebrow">{t('Human questions. Shared mathematical knowledge.')}</p>
          <h1>{t('Mathematics, reviewed and carried forward.')}</h1>
          <p className="lede">
            {t(
              'Bring a paper, a conjecture or a formal proof. Mathesis brings people and AI together to review new mathematics and build a connected body of knowledge, grounded in Lean.',
            )}
          </p>
          <p className="home-reading">
            {t(
              'Papers and written conjectures are compared with existing knowledge. Accepted work receives credit and a path toward formalization; reusable Lean proofs become foundations for what comes next.',
            )}
          </p>
          <nav className="home-actions" aria-label={t('About Mathesis')}>
            <Link to={{ kind: 'research_graph' }}>{t('Explore the knowledge graph')}</Link>
            <Link to={{ kind: 'policy' }}>{t('How review works')}</Link>
          </nav>
        </div>
        <ResearchCycle />
      </header>

      <section className="home-participation" aria-labelledby="participate-heading">
        <h2 id="participate-heading">{t('Take part in the research')}</h2>
        <div className="home-paths">
          <article>
            <h3>
              <Link to={{ kind: 'submit' }}>{t('Submit a paper or note')}</Link>
            </h3>
            <p>
              {t(
                'Receive an acceptance decision, analysis and next steps, including results worth formalizing. Accepted work can be published with its authors and sources.',
              )}
            </p>
          </article>
          <article>
            <h3>
              <Link to={{ kind: 'submit', intake: 'conjecture' }}>{t('Propose a conjecture')}</Link>
            </h3>
            <p>
              {t(
                'State a precise question and its background. Review checks what is already known and what new mathematics a solution would contribute.',
              )}
            </p>
          </article>
          <article>
            <h3>
              <Link to={{ kind: 'conjectures' }}>{t('Work on an open problem')}</Link>
            </h3>
            <p>
              {t(
                'Build on someone’s question. Submit a written solution for review, or contribute a Lean proof or counterexample to a confirmed statement.',
              )}
            </p>
          </article>
          <article>
            <h3>
              <Link to={{ kind: 'formalizations' }}>{t('Contribute a formalization')}</Link>
            </h3>
            <p>
              {t(
                'Formalize a result or a useful intermediate lemma. Share Lean sources, including a link to existing code, with credit for the mathematics and its formalization.',
              )}
            </p>
          </article>
        </div>
        <p className="home-invitation">
          {t('Currently welcoming researchers by invitation.')}{' '}
          <Link to={{ kind: 'contribute' }}>{t('Participation and access')}</Link>
        </p>
      </section>

      <section aria-labelledby="recent-heading" className="home-research">
        <div className="section-heading">
          <h2 id="recent-heading">{t('Latest research')}</h2>
          <Link to={{ kind: 'papers' }}>{t('All research')}</Link>
        </div>
        <Async state={papers.state} onRetry={papers.reload}>
          {(listing) =>
            listing.items.length ? (
              <ul className="home-papers">
                {listing.items.slice(0, 3).map((paper) => (
                  <li key={paper.record}>
                    <article>
                      <h3>
                        <Link to={{ kind: 'paper', record: paper.record }}>
                          <InlineLatex source={paper.title} />
                        </Link>
                      </h3>
                      <div className="entry-meta">
                        <AuthorLine authors={paper.authors.map(({ name }) => ({ name }))} />
                        {paper.accepted_at ? <DateText iso={paper.accepted_at} /> : null}
                      </div>
                    </article>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">{t('Accepted public papers and notes will appear here.')}</p>
            )
          }
        </Async>
      </section>
      <nav className="home-browse" aria-label={t('More research')}>
        <Link to={{ kind: 'conjectures' }}>{t('Open problems')}</Link>
        <details className="home-subjects">
          <summary>{t('Browse by subject')}</summary>
          <ul>
            {SUBJECTS.map(([msc, title]) => (
              <li key={msc}>
                <Link to={{ kind: 'papers', msc }}>{t(title)}</Link>
              </li>
            ))}
          </ul>
        </details>
      </nav>
    </div>
  );
}
