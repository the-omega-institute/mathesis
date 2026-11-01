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
      <header className="home-hero">
        <p className="eyebrow">Mathesis · mathematical research review</p>
        <h1>Mathematics, reviewed and carried forward.</h1>
        <p className="lede">
          Mathesis brings AI review, human mathematical judgment, computation, and formal proof into
          one public record. Submit a manuscript to receive a clear decision, an analysis of its
          arguments, and a path for what can be done next.
        </p>
        <p className="home-hero-note">
          A result enters as a claim. It can leave with a decision, evidence, credit, and a place in
          the next proof.
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

      <section className="home-purpose" aria-labelledby="purpose-heading">
        <div>
          <p className="eyebrow">What Mathesis does</p>
          <h2 id="purpose-heading">One review loop for the life of a result.</h2>
        </div>
        <p>
          AI reads the manuscript, checks its reasoning against the source and existing knowledge,
          and runs the available computational checks. Authors and mathematicians keep the claim,
          context, and credit. Accepted work can continue into formalization and new research.
        </p>
      </section>

      <ol className="home-flow" aria-label="The Mathesis review loop">
        <li>
          <span className="home-flow-number">01</span>
          <h3>Review</h3>
          <p>Read the arguments and decide whether the result is ready to be accepted.</p>
        </li>
        <li>
          <span className="home-flow-number">02</span>
          <h3>Check</h3>
          <p>Compare the claims with the manuscript, calculations, sources, and prior work.</p>
        </li>
        <li>
          <span className="home-flow-number">03</span>
          <h3>Formalize</h3>
          <p>Identify statements that deserve a Lean proof and connect them to existing work.</p>
        </li>
        <li>
          <span className="home-flow-number">04</span>
          <h3>Continue</h3>
          <p>Turn accepted results into credited stepping stones for the next question.</p>
        </li>
      </ol>

      <section className="home-outcomes" aria-labelledby="outcomes-heading">
        <div>
          <p className="eyebrow">What you receive</p>
          <h2 id="outcomes-heading">A decision with useful mathematical detail.</h2>
        </div>
        <ul>
          <li>
            <strong>Accepted or not accepted</strong>
            <span>A direct publication decision, with the reason made clear.</span>
          </li>
          <li>
            <strong>Review analysis</strong>
            <span>Arguments, evidence, computation, and limits explained in plain language.</span>
          </li>
          <li>
            <strong>Further work</strong>
            <span>Formalization targets and distinct directions worth exploring next.</span>
          </li>
        </ul>
      </section>

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
