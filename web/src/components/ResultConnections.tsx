import type { PublicClaim, PublicPaper } from '../api/types';
import { Link } from '../routing/router';
import { resultLabel } from '../lib/statements';
import { isOpenKind } from '../lib/labels';
import { InlineLatex } from './Markdown';

/** A direct neighborhood of the accepted manuscript's declared graph.
 * A reader follows a named result, then sees its own neighborhood. */
export function ResultConnections({ paper, claim }: { paper: PublicPaper; claim: PublicClaim }) {
  const dependencies = paper.claims?.filter((item) => claim.depends_on.includes(item.id)) ?? [];
  const usedBy = paper.claims?.filter((item) => item.depends_on.includes(claim.id)) ?? [];
  if (!dependencies.length && !usedBy.length) return null;
  const uses = dependencies.filter((item) => !isOpenKind(item.kind));
  const assumptions = dependencies.filter((item) => isOpenKind(item.kind));
  const links = (items: PublicClaim[]) => (
    <ul>
      {items.map((item) => (
        <li key={item.id}>
          <Link to={{ kind: 'result', record: paper.summary.record, claim: item.id }}>
            <InlineLatex source={resultLabel(item)} />
          </Link>
        </li>
      ))}
    </ul>
  );
  return (
    <section className="result-relations">
      <h2>Connections within this paper</h2>
      <p className="small muted">Dependencies stated in the manuscript.</p>
      <div className="result-neighborhood">
        <div>
          {assumptions.length ? (
            <>
              <h3>Conditional on these assumptions</h3>
              {links(assumptions)}
            </>
          ) : null}
          {uses.length ? (
            <>
              <h3>Uses these results</h3>
              {links(uses)}
            </>
          ) : null}
          {!dependencies.length ? (
            <p className="small muted">No prerequisites recorded here.</p>
          ) : null}
        </div>
        <div className="result-neighborhood-current" aria-current="true">
          <span className="small muted">This result</span>
          <p>
            <InlineLatex source={resultLabel(claim)} />
          </p>
        </div>
        <div>
          {usedBy.length ? (
            <>
              <h3>
                {isOpenKind(claim.kind) ? 'Assumed in these statements' : 'Used in these results'}
              </h3>
              {links(usedBy)}
            </>
          ) : (
            <p className="small muted">No later uses recorded here.</p>
          )}
        </div>
      </div>
    </section>
  );
}
