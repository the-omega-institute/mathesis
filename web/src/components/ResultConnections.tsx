import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { ResultUse } from '../api/types';
import { EntrantName } from '../pages/Leaderboard';
import { ProofAttribution } from './ProofAttribution';
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

/** Actual checked contribution use; independent of manuscript-declared links above. */
export function CheckedResultUses({ record, claim }: { record: string; claim: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.resultUses(record, claim, { signal }),
    [api, record, claim],
  );
  const uses = useAsync(load);
  if (!uses.value?.length) return null;
  const outgoing = uses.value.filter(
    (edge) => edge.result.record === record && edge.result.claim === claim,
  );
  const incoming = uses.value.filter(
    (edge) => edge.used_result.record === record && edge.used_result.claim === claim,
  );
  const list = (edges: ResultUse[], forward: boolean) => (
    <ul>
      {edges.map((edge) => {
        const other = forward ? edge.used_result : edge.result;
        const label = other.label.trim().replace(/\s*\[[^\]]*\]\s*$/, '');
        return (
          <li key={`${edge.attempt}:${edge.source_attempt}`}>
            <Link to={{ kind: 'result', record: other.record, claim: other.claim }}>
              <InlineLatex source={`${label} — ${other.title}`} />
            </Link>{' '}
            · version {other.version}
            <p className="small">{other.authors.map((a) => a.name).join(', ')}</p>
            <p>
              {forward ? 'Uses' : 'Used the checked proposition'} “{edge.contribution.title}”,
              contributed by <EntrantName entrant={edge.contributor} /> ·{' '}
              {edge.contribution.license}.
            </p>
            <ProofAttribution attribution={edge.attribution ?? undefined} />
            <p className="small">
              Evidence: the named proposition occurs in the independently checked proof term. The
              complete checked source context contains {edge.joint_context.length} contributed
              proposition{edge.joint_context.length === 1 ? '' : 's'}; individual sufficiency is not
              asserted.
            </p>
            <a
              href={`/api/v1/papers/${encodeURIComponent(edge.result.record)}/statements/${encodeURIComponent(edge.result.claim)}/attempts/${encodeURIComponent(edge.attempt)}/bundle`}
            >
              Read proof sources and verification details
            </a>
          </li>
        );
      })}
    </ul>
  );
  return (
    <section className="result-relations">
      <h2>Checked contribution use</h2>
      <p>
        These links concern the exact named propositions contributed during proof work. The source
        result itself may remain unfinished. Citations and unused imports do not establish checked
        use.
      </p>
      {outgoing.length ? (
        <>
          <h3>Uses propositions from these results</h3>
          {list(outgoing, true)}
        </>
      ) : null}
      {incoming.length ? (
        <>
          <h3>Propositions used in these results</h3>
          {list(incoming, false)}
        </>
      ) : null}
    </section>
  );
}
