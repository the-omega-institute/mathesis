import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { InlineLatex } from '../components/Markdown';
import { Async } from '../components/ui';
import { Link } from '../routing/router';
import { PaperEntry } from './Papers';
import '../styles/home.css';

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
        <h1 id="intro-heading">An open record of mathematical research.</h1>
        <p className="lede">
          AI review, publication of accepted research, and further work toward formal proofs.
        </p>
        <div className="intro-actions">
          <Link to={{ kind: 'papers' }} className="button">
            Explore the archive
          </Link>
          <Link to={{ kind: 'submit' }} className="text-link">
            Submit research
          </Link>
        </div>
      </section>

      <div className="archive-main">
        <section aria-labelledby="recent-heading" className="archive-section">
          <div className="section-heading">
            <h2 id="recent-heading">Latest research</h2>
            <Link to={{ kind: 'papers' }} className="text-link">
              Browse all
            </Link>
          </div>
          <Async state={papers.state} onRetry={papers.reload}>
            {(listing) =>
              listing.items.length === 0 ? (
                <p className="archive-empty">Accepted public papers and notes will appear here.</p>
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

        <section aria-labelledby="open-heading" className="archive-section archive-conjectures">
          <div className="section-heading">
            <h2 id="open-heading">Open conjectures</h2>
            <Link to={{ kind: 'conjectures' }} className="text-link">
              View all
            </Link>
          </div>
          <Async state={conjectures.state} onRetry={conjectures.reload}>
            {(listing) => {
              const open = listing.items.filter((c) => c.status === 'open').slice(0, 3);
              return open.length ? (
                <ul className="entry-list archive-entries">
                  {open.map((c) => (
                    <li key={`${c.record}:${c.claim}`}>
                      <article>
                        <Link
                          className="entry-title"
                          to={{ kind: 'conjecture', record: c.record, claim: c.claim }}
                        >
                          <InlineLatex source={c.title} />
                        </Link>
                        <p className="entry-meta">
                          <code>{c.record}</code> · {c.claim}
                        </p>
                      </article>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="archive-empty">There are no open conjectures at present.</p>
              );
            }}
          </Async>
        </section>
      </div>

      <nav className="archive-resources" aria-label="Research resources">
        <Link to={{ kind: 'submit', intake: 'lean' }}>Contribute a formalization</Link>
        <Link to={{ kind: 'submit', intake: 'review' }}>Request a private review</Link>
        <Link to={{ kind: 'policy' }}>Review policy</Link>
      </nav>
    </div>
  );
}
