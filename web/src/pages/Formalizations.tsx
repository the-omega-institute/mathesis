import { InlineLatex } from '../components/Markdown';
import { useCallback, useState, type FormEvent } from 'react';
import { useApi } from '../api/context';
import { usePaged } from '../api/usePaged';
import { Async, Badge } from '../components/ui';
import { LatexText } from '../components/Markdown';
import { ProofAttribution } from '../components/ProofAttribution';
import { Pager } from '../components/Pager';
import { Link } from '../routing/router';
import { AuthorLine } from '../components/AuthorLine';
import { EntrantName } from './Leaderboard';

export function FormalizationsPage() {
  const api = useApi();
  const [status, setStatus] = useState('open');
  const [kind, setKind] = useState('');
  const [draft, setDraft] = useState('');
  const [query, setQuery] = useState('');
  const load = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.formalizationOpportunities(
        { before, limit: 20, status, kind: kind || undefined, q: query || undefined },
        { signal },
      ),
    [api, status, kind, query],
  );
  const paged = usePaged(load);
  const search = (event: FormEvent) => {
    event.preventDefault();
    setQuery(draft.trim());
  };
  return (
    <div className="page discovery-page">
      <header className="page-head">
        <div>
          <p className="eyebrow">Participate in mathematical research</p>
          <h1>Formalizations</h1>
          <p className="lede">Turn an argument into a checked proof.</p>
          <p>
            Formalize a published result, prove or disprove a conjecture, or contribute existing
            Lean work with its original attribution.
          </p>
        </div>
        <Link className="button" to={{ kind: 'submit', intake: 'lean' }}>
          Submit Lean work <span aria-hidden="true">↗</span>
        </Link>
      </header>
      <div className="formalization-intro">
        <p>
          <strong>01 · Choose a statement</strong>
          <span>Browse exact targets confirmed by their authors.</span>
        </p>
        <p>
          <strong>02 · Contribute your proof</strong>
          <span>Submit Lean sources and credit the people behind them.</span>
        </p>
        <p>
          <strong>03 · Get a checked receipt</strong>
          <span>Verified proofs appear with their source research.</span>
        </p>
      </div>
      <div className="filter-row" role="group" aria-label="Formalization status">
        {[
          ['open', 'Ready to formalize'],
          ['verified', 'Verified proofs'],
          ['disproved', 'Disproofs'],
        ].map(([value, label]) => (
          <button
            key={value}
            className="filter-chip"
            aria-pressed={status === value}
            onClick={() => setStatus(value!)}
          >
            {label}
          </button>
        ))}
      </div>
      <form className="formalization-search" role="search" onSubmit={search}>
        <label htmlFor="formalization-query" className="visually-hidden">
          Find a statement
        </label>
        <input
          id="formalization-query"
          type="search"
          maxLength={200}
          placeholder="Search statements, papers, or authors"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <label htmlFor="formalization-kind" className="visually-hidden">
          Statement type
        </label>
        <select id="formalization-kind" value={kind} onChange={(e) => setKind(e.target.value)}>
          <option value="">All statements</option>
          <option value="paper">Paper results</option>
          <option value="conjecture">Conjectures</option>
        </select>
        <button className="button button-quiet">Search</button>
      </form>
      <Async state={paged.state} onRetry={paged.reload}>
        {(page) =>
          page.items.length ? (
            <ul className="entry-list">
              {page.items.map((item) => (
                <li key={`${item.record}:${item.claim}`}>
                  <article>
                    <p className="entry-meta">
                      <code>{item.record}</code>
                      <span>{item.label}</span>
                      <Badge tone={item.status === 'open' ? 'muted' : 'good'}>
                        {item.status === 'open'
                          ? 'Target ready'
                          : item.status === 'verified'
                            ? 'Lean ✓'
                            : 'Lean disproof'}
                      </Badge>
                    </p>
                    <Link
                      className="entry-title"
                      to={{ kind: 'statement', record: item.record, claim: item.claim }}
                    >
                      <InlineLatex source={item.title} />
                    </Link>
                    <AuthorLine authors={item.authors} />
                    <LatexText source={item.statement} />
                    {item.contributor ? (
                      <p className="small">
                        Verified contribution by <EntrantName entrant={item.contributor} /> ·{' '}
                        {item.verified_count} verified{' '}
                        {item.verified_count === 1 ? 'attempt' : 'attempts'}
                      </p>
                    ) : null}
                    <ProofAttribution attribution={item.attribution ?? undefined} />
                    <div className="button-row">
                      <Link
                        className="button button-small"
                        to={{
                          kind: 'submit',
                          intake: 'lean',
                          record: item.record,
                          claim: item.claim,
                        }}
                      >
                        {item.status === 'open' ? 'Contribute a proof' : 'Add another proof'}
                      </Link>
                      <Link
                        className="text-link"
                        to={{ kind: 'statement', record: item.record, claim: item.claim }}
                      >
                        View statement & {item.status === 'open' ? 'target' : 'receipt'} →
                      </Link>
                    </div>
                  </article>
                </li>
              ))}
            </ul>
          ) : (
            <div className="empty-state">
              <h2>
                {query || kind
                  ? 'No matching statements'
                  : status === 'open'
                    ? 'The next proof can start with you.'
                    : 'Verified contributions will appear here.'}
              </h2>
              <p>
                {status === 'open'
                  ? 'Targets appear here after author approval and confirmation. You can already submit an existing formalization with its authors, source, and license.'
                  : 'Only independent verification of the current confirmed target makes a contribution public here.'}
              </p>
              <Link className="text-link" to={{ kind: 'submit', intake: 'lean' }}>
                Submit existing Lean work →
              </Link>
            </div>
          )
        }
      </Async>
      <Pager paged={paged} />
      <p className="hint">
        For agents: use the formalization-opportunities MCP tool or{' '}
        <code>mathesis-contribute formalization-opportunities</code>.
      </p>
    </div>
  );
}
