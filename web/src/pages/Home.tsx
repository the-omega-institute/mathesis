import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async, DateText } from '../components/ui';
import { AuthorLine } from '../components/AuthorLine';
import { InlineLatex } from '../components/Markdown';
import { ResearchCycle } from '../components/ResearchCycle';
import { Link, useRouter } from '../routing/router';
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
  const { navigate } = useRouter();
  const [query, setQuery] = useState('');
  const loadPapers = useCallback(
    (signal: AbortSignal) => api.listPapers({ limit: 3 }, { signal }),
    [api],
  );
  const papers = useAsync(loadPapers);
  return (
    <div className="page home">
      <header className="home-hero">
        <div className="home-introduction">
          <p className="eyebrow">Mathesis · mathematical research review</p>
          <h1>Mathematics, reviewed and carried forward.</h1>
          <p className="lede">
            Mathematicians ask the questions and build on one another’s results. Mathesis supports
            their work with AI review, computation and formal proof. Accepted research becomes part
            of a shared knowledge base, with credit for its authors and the work it builds on.
          </p>
        </div>
        <ResearchCycle />
        <form
          className="home-search"
          role="search"
          aria-label="Search research"
          onSubmit={(event) => {
            event.preventDefault();
            navigate({ kind: 'papers', q: query.trim() });
          }}
        >
          <label htmlFor="home-query">Search the archive</label>
          <div className="archive-search-input">
            <input
              id="home-query"
              type="search"
              maxLength={300}
              placeholder="Title, author, abstract, DOI or record"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            <button type="submit" className="button">
              Search
            </button>
          </div>
        </form>
        <nav className="home-actions" aria-label="Research and submission">
          <Link to={{ kind: 'submit' }} className="button">
            Submit a manuscript
          </Link>
          <Link to={{ kind: 'papers' }} className="text-link">
            Explore accepted research
          </Link>
          <Link to={{ kind: 'formalizations' }} className="text-link">
            Contribute a formal proof
          </Link>
          <Link to={{ kind: 'policy' }} className="text-link">
            Review policy
          </Link>
        </nav>
      </header>

      <section className="home-overview" aria-labelledby="overview-heading">
        <h2 id="overview-heading">From review to further research</h2>
        <dl>
          <div>
            <dt>A clear decision</dt>
            <dd>
              Acceptance or rejection, with reasons. Accepted work can enter the public archive.
            </dd>
          </div>
          <div>
            <dt>Analysis and next steps</dt>
            <dd>A review of the arguments and evidence, with suggestions and further questions.</dd>
          </div>
          <div>
            <dt>Formal proof and credit</dt>
            <dd>
              Identify what to formalize in Lean and connect results to earlier work, crediting each
              contribution.
            </dd>
          </div>
        </dl>
      </section>

      <section aria-labelledby="recent-heading" className="home-research">
        <div className="section-heading">
          <h2 id="recent-heading">Latest research</h2>
          <Link to={{ kind: 'papers' }}>All research</Link>
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
              <p className="muted">Accepted public papers and notes will appear here.</p>
            )
          }
        </Async>
      </section>
      <nav className="home-browse" aria-label="More research">
        <Link to={{ kind: 'conjectures' }}>Open problems</Link>
        <details className="home-subjects">
          <summary>Browse by subject</summary>
          <ul>
            {SUBJECTS.map(([msc, title]) => (
              <li key={msc}>
                <Link to={{ kind: 'papers', msc }}>{title}</Link>
              </li>
            ))}
          </ul>
        </details>
      </nav>
    </div>
  );
}
