import type { PublicClaim, PublicPaper } from '../api/types';
import { Link } from '../routing/router';
import { resultLabel } from '../lib/statements';
import { InlineLatex, LatexText } from './Markdown';

export function PaperResults({ paper }: { paper: PublicPaper }) {
  const claims = paper.claims ?? [];
  const main = claims.filter((claim) => claim.role === 'main');
  const supporting = claims.filter((claim) => claim.role !== 'main');
  const list = (items: PublicClaim[]) => (
    <ul className="result-list">
      {items.map((claim) => (
        <li key={claim.id} id={`public-${claim.id}`}>
          <h3>
            <Link to={{ kind: 'result', record: paper.summary.record, claim: claim.id }}>
              <InlineLatex source={resultLabel(claim)} />
            </Link>
          </h3>
          <LatexText source={claim.statement} className="result-excerpt" />
          {claim.verified || claim.lean ? (
            <p className="small">A verified formal proof is available.</p>
          ) : null}
        </li>
      ))}
    </ul>
  );
  return (
    <section className="paper-results" aria-labelledby="results-h">
      <h2 id="results-h">Results</h2>
      {list(main)}
      {supporting.length ? (
        <details>
          <summary>Supporting statements ({supporting.length})</summary>
          {list(supporting)}
        </details>
      ) : null}
    </section>
  );
}
