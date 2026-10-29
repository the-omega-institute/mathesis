import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async } from '../components/ui';
import { Link } from '../routing/router';
import { PaperEntry } from './Papers';
import { ConjectureEntry } from './Conjectures';
import '../styles/home.css';

function Arrow() {
  return <span aria-hidden="true">→</span>;
}

export function HomePage() {
  const api = useApi();
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
      <section className="archive-intro" aria-labelledby="intro-heading">
        <p className="eyebrow">An open home for mathematical research</p>
        <h1 id="intro-heading">Where mathematics takes form.</h1>
        <p className="lede">
          Submit a paper, short note, or conjecture. Mathesis gives it a deep research review and a
          durable home for the mathematics that follows.
        </p>
        <div className="intro-actions">
          <Link to={{ kind: 'submit' }} className="button">
            Submit a paper or conjecture <Arrow />
          </Link>
          <Link to={{ kind: 'papers' }} className="button button-quiet">
            Browse papers <Arrow />
          </Link>
          <Link to={{ kind: 'submit', intake: 'lean' }} className="button button-quiet">
            Contribute a formalization <Arrow />
          </Link>
        </div>
      </section>

      <div className="archive-layout">
        <div className="archive-main">
          <section aria-labelledby="recent-heading" className="archive-section">
            <div className="section-heading">
              <div>
                <h2 id="recent-heading">Latest research</h2>
                <p>Recently accepted papers, notes, and conjectures.</p>
              </div>
              <Link to={{ kind: 'papers' }} className="text-link">
                Browse all <Arrow />
              </Link>
            </div>
            <Async state={papers.state} onRetry={papers.reload}>
              {(listing) =>
                listing.items.length === 0 ? (
                  <div className="archive-empty">
                    <h3>Share your first contribution.</h3>
                    <p>Accepted public research will appear here.</p>
                    <Link to={{ kind: 'submit' }} className="text-link">
                      Submit research <Arrow />
                    </Link>
                  </div>
                ) : (
                  <ul className="entry-list archive-entries">
                    {listing.items.map((p) => (
                      <li key={p.record}>
                        <PaperEntry paper={p} />
                      </li>
                    ))}
                  </ul>
                )
              }
            </Async>
          </section>

          <section aria-labelledby="open-heading" className="archive-section">
            <div className="section-heading">
              <div>
                <h2 id="open-heading">Open conjectures</h2>
                <p>Research questions published for others to explore.</p>
              </div>
              <Link to={{ kind: 'conjectures' }} className="text-link">
                View all <Arrow />
              </Link>
            </div>
            <Async state={conjectures.state} onRetry={conjectures.reload}>
              {(listing) => {
                const open = listing.items.filter((c) => c.status === 'open').slice(0, 3);
                return open.length ? (
                  <ul className="entry-list archive-entries">
                    {open.map((c) => (
                      <li key={`${c.record}:${c.claim}`}>
                        <ConjectureEntry conjecture={c} />
                      </li>
                    ))}
                  </ul>
                ) : (
                  <div className="archive-empty">
                    <h3>A question can be a contribution, too.</h3>
                    <p>
                      No open conjectures yet. Submit a standalone conjecture with its statement and
                      background.
                    </p>
                    <Link to={{ kind: 'submit' }} className="text-link">
                      Submit a conjecture <Arrow />
                    </Link>
                  </div>
                );
              }}
            </Async>
          </section>
        </div>

        <aside className="archive-guide" aria-labelledby="authors-heading">
          <p className="eyebrow">Take part</p>
          <h2 id="authors-heading">Every proof builds on something.</h2>
          <p>Bring a new result, an open question, or a formal proof of someone else’s work.</p>
          <Link to={{ kind: 'formalizations' }} className="text-link">
            Find a statement to formalize <Arrow />
          </Link>
          <div className="archive-review-note">
            <h3>Before publication</h3>
            <p>
              Submissions are reviewed. Accepted work receives a permanent record and appears
              publicly when the author chooses to share it.
            </p>
            <Link to={{ kind: 'policy' }} className="text-link">
              How review works <Arrow />
            </Link>
          </div>
        </aside>
      </div>
    </div>
  );
}
