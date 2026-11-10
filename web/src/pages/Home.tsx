import { t } from '../i18n/locale';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { ResearchCycle } from '../components/ResearchCycle';
import { mergeCollectedNotes } from '../lib/projectNotes';
import { researchEntries } from '../lib/researchEntries';
import { homeSubjects } from '../lib/subjects';
import { PublishedNoteEntry } from './PublishedNote';
import { PaperEntry } from './Papers';
import { Link } from '../routing/router';
import '../styles/home.css';
import '../styles/subjects.css';

export function HomePage() {
  const api = useApi();
  const loadPapers = useCallback(
    (signal: AbortSignal) => api.listPapers({ limit: 3 }, { signal }),
    [api],
  );
  const papers = useAsync(loadPapers);
  const collected = useAsync(
    useCallback((signal: AbortSignal) => api.listCollectedPapers({ limit: 25 }, { signal }), [api]),
  );
  const recent = researchEntries(
    papers.value?.items ?? [],
    mergeCollectedNotes(collected.value?.items ?? []),
  ).slice(0, 3);
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
          <p className="home-primary-action">
            <Link className="button home-submit" to={{ kind: 'submit' }}>
              {t('Submit a paper')}
            </Link>
          </p>
          <p className="home-reading">
            {t(
              'Papers and written conjectures are compared with existing knowledge. Accepted work receives credit and a path toward formalization; reusable Lean proofs become foundations for what comes next.',
            )}
          </p>
          <nav className="home-actions" aria-label={t('About Mathesis')}>
            <Link to={{ kind: 'learn_lean' }}>{t('Get familiar with Lean formalization')}</Link>
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
          {t('Researcher access is currently open.')}{' '}
          <Link to={{ kind: 'contribute' }}>{t('Participation and access')}</Link>
        </p>
      </section>

      <section aria-labelledby="recent-heading" className="home-research">
        <div className="section-heading">
          <h2 id="recent-heading">{t('Latest research')}</h2>
          <Link to={{ kind: 'papers' }}>{t('All research')}</Link>
        </div>
        <ul className="home-papers">
          {recent.map((entry) =>
            entry.kind === 'note' ? (
              <li key={`note:${entry.note.id}`}>
                <PublishedNoteEntry note={entry.note} />
              </li>
            ) : (
              <li key={`paper:${entry.paper.record}`}>
                <PaperEntry paper={entry.paper} />
              </li>
            ),
          )}
        </ul>
      </section>
      <section className="home-subjects" aria-labelledby="subjects-heading">
        <div className="section-heading">
          <h2 id="subjects-heading">{t('Browse by subject')}</h2>
          <Link to={{ kind: 'subjects' }}>{t('All subjects')}</Link>
        </div>
        <ul className="subject-links">
          {homeSubjects.map((subject) => (
            <li key={subject.code}>
              <Link to={{ kind: 'subject', msc: subject.code }}>{t(subject.title)}</Link>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
