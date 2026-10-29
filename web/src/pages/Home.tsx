import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async } from '../components/ui';
import { Link, useRouter } from '../routing/router';
import { PaperEntry } from './Papers';
import { ConjectureEntry } from './Conjectures';
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
    (signal: AbortSignal) => api.listPapers({ limit: 5 }, { signal }),
    [api],
  );
  const papers = useAsync(loadPapers);
  const loadConjectures = useCallback(
    (signal: AbortSignal) => api.listConjectures({ limit: 3, status: 'open' }, { signal }),
    [api],
  );
  const conjectures = useAsync(loadConjectures);
  return (
    <div className="page home">
      <header className="archive-intro">
        <h1>Mathematical research</h1>
        <p className="lede">
          An archive of papers, conjectures and formal proofs. Submit a manuscript for AI review, a
          publication decision and suggestions for further research.
        </p>
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
        <nav className="intro-actions" aria-label="Research and submission">
          <Link to={{ kind: 'submit' }}>Submit a paper or conjecture</Link>
          <Link to={{ kind: 'formalizations' }}>Contribute a formal proof</Link>
          <Link to={{ kind: 'policy' }}>Review policy</Link>
        </nav>
      </header>
      <section className="subject-browse" aria-labelledby="subjects-h">
        <h2 id="subjects-h">Browse by subject</h2>
        <ul>
          {SUBJECTS.map(([msc, title]) => (
            <li key={msc}>
              <Link to={{ kind: 'papers', msc }}>{title}</Link>
            </li>
          ))}
        </ul>
        <Link to={{ kind: 'papers' }}>All subjects and advanced search</Link>
      </section>
      <section aria-labelledby="recent-heading" className="archive-section">
        <div className="section-heading">
          <h2 id="recent-heading">Latest research</h2>
          <Link to={{ kind: 'papers' }}>Browse all</Link>
        </div>
        <Async state={papers.state} onRetry={papers.reload}>
          {(listing) =>
            listing.items.length ? (
              <ul className="entry-list archive-entries">
                {listing.items.map((paper) => (
                  <li key={paper.record}>
                    <PaperEntry paper={paper} />
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">Accepted public papers and notes will appear here.</p>
            )
          }
        </Async>
      </section>
      <section aria-labelledby="open-heading" className="archive-section">
        <div className="section-heading">
          <h2 id="open-heading">Open conjectures</h2>
          <Link to={{ kind: 'conjectures' }}>Browse all</Link>
        </div>
        <Async state={conjectures.state} onRetry={conjectures.reload}>
          {(listing) =>
            listing.items.length ? (
              <ul className="entry-list archive-entries">
                {listing.items
                  .filter((item) => item.status === 'open')
                  .slice(0, 3)
                  .map((item) => (
                    <li key={`${item.record}:${item.claim}`}>
                      <ConjectureEntry conjecture={item} />
                    </li>
                  ))}
              </ul>
            ) : (
              <p className="muted">Accepted open problems will appear here.</p>
            )
          }
        </Async>
      </section>
    </div>
  );
}
